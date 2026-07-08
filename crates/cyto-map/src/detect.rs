use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, bail};
use binseq::BinseqReader;
use log::{info, warn};
use parking_lot::Mutex;

use crate::geometry::{Component, Geometry, Read, ReadMate, Region};
use crate::mapper::{CrisprMapper, GexMapper, ProbeMapper, Unpositioned, WhitelistMapper};
use cyto_cli::map::MultiPairedInput;

/// UMI length for all current Flex chemistries (V1 and V2).
const FLEX_UMI_LENGTH: usize = 12;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for geometry auto-detection.
pub struct DetectionConfig {
    pub num_reads: usize,
    pub min_proportion: f64,
    /// Minimum proportion of sampled reads a position must match to count as
    /// significant when estimating the remap window (default 0.01 = 1%).
    pub remap_min_proportion: f64,
    /// Number of worker threads for parallel record sampling.
    pub num_threads: usize,
}

/// Evidence for a single component's detected position.
#[derive(Debug)]
pub struct ComponentEvidence {
    pub component: Component,
    pub mate: ReadMate,
    pub position: usize,
    pub seq_len: Option<usize>,
    pub match_count: usize,
    pub match_proportion: f64,
    /// Sum of hits across `[position - remap_window, position + remap_window]`
    /// on the same mate. For references that are largely unique within the
    /// window (the typical case: ≥16bp whitelist barcodes, ≥50bp gex probes),
    /// this closely tracks the number of reads `cyto map --remap-window N`
    /// would score for this component. For SHORT references -- notably the
    /// 8bp `[probe]` multiplex barcode in Flex chemistry -- a single read can
    /// contribute hits at multiple positions within the window, so
    /// `windowed_match_count` can EXCEED `total_reads_sampled` (and
    /// `windowed_match_proportion` can exceed 1.0). In that regime the value
    /// is an UPPER BOUND on what `cyto map` would score, not an exact count.
    /// The window scope is the single global `DetectionResult::remap_window`
    /// (or the per-file `remap_window` on `PerFileResult::evidence`).
    pub windowed_match_count: usize,
    /// `windowed_match_count` as a fraction of `total_reads_sampled`. May
    /// exceed 1.0 for short references; see `windowed_match_count` docs.
    pub windowed_match_proportion: f64,
    /// Top positions by match count (for logging alternative candidates).
    pub top_positions: Vec<(ReadMate, usize, usize)>,
}

/// Detection result for a single input file/lane.
#[derive(Debug)]
pub struct PerFileResult {
    pub label: String,
    pub geometry_string: String,
    pub remap_window: usize,
    pub evidence: Vec<ComponentEvidence>,
    pub total_reads_sampled: usize,
}

/// Full detection result.
#[derive(Debug)]
pub struct DetectionResult {
    pub geometry: Geometry,
    pub geometry_string: String,
    pub remap_window: usize,
    pub evidence: Vec<ComponentEvidence>,
    pub total_reads_sampled: usize,
    pub per_file_results: Vec<PerFileResult>,
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Detection mode: which components to scan for.
#[derive(Debug, Clone, Copy)]
enum DetectionMode {
    Gex,
    Crispr,
}

/// Accumulates match counts per (component, mate, position).
#[derive(Default, Clone)]
struct PositionAccumulator {
    counts: HashMap<(Component, ReadMate, usize), usize>,
    total_reads: usize,
}

impl PositionAccumulator {
    fn record_position(&mut self, component: Component, mate: ReadMate, pos: usize) {
        *self.counts.entry((component, mate, pos)).or_insert(0) += 1;
    }

    fn merge_from(&mut self, other: &Self) {
        for (&key, &count) in &other.counts {
            *self.counts.entry(key).or_insert(0) += count;
        }
        self.total_reads += other.total_reads;
    }

    /// Sum hits across
    /// `[best_pos.saturating_sub(window), best_pos.saturating_add(window)]`
    /// on the given `(component, mate)`. Mirrors
    /// `seqhash::query_at_with_remap` edge-case behavior: when
    /// `best_pos < window`, the lower bound clips to 0. The upper bound
    /// saturates symmetrically; in practice `best_pos + window` cannot
    /// overflow `usize` (both are bounded by read length), so this is
    /// purely defensive.
    fn windowed_count(
        &self,
        component: Component,
        mate: ReadMate,
        best_pos: usize,
        window: usize,
    ) -> usize {
        let lo = best_pos.saturating_sub(window);
        let hi = best_pos.saturating_add(window);
        self.counts
            .iter()
            .filter(|((c, m, p), _)| *c == component && *m == mate && *p >= lo && *p <= hi)
            .map(|(_, &count)| count)
            .sum()
    }
}

// ---------------------------------------------------------------------------
// GEX detection processor
// ---------------------------------------------------------------------------

/// Shared state for GEX detection processors (works across clones).
struct GexSharedState {
    whitelist: WhitelistMapper<Unpositioned>,
    gex: GexMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    global_accumulator: Mutex<PositionAccumulator>,
    counter: AtomicUsize,
    limit: usize,
}

/// Processor for GEX geometry detection.
struct GexDetectionProcessor {
    shared: Arc<GexSharedState>,
    local: PositionAccumulator,
    tid: usize,
}

impl Clone for GexDetectionProcessor {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            local: PositionAccumulator::default(),
            tid: self.tid,
        }
    }
}

impl GexDetectionProcessor {
    fn scan_record(&mut self, r1_seq: &[u8], r2_seq: &[u8]) {
        if self.shared.counter.fetch_add(1, Ordering::Relaxed) >= self.shared.limit {
            return;
        }
        self.local.total_reads += 1;

        for (seq, mate) in [(r1_seq, ReadMate::R1), (r2_seq, ReadMate::R2)] {
            for pos in self.shared.whitelist.scan_positions(seq) {
                self.local.record_position(Component::Barcode, mate, pos);
            }
            for pos in self.shared.gex.scan_positions(seq) {
                self.local.record_position(Component::Gex, mate, pos);
            }
            if let Some(ref probe) = self.shared.probe {
                for pos in probe.scan_positions(seq) {
                    self.local.record_position(Component::Probe, mate, pos);
                }
            }
        }
    }

    fn flush(&mut self) {
        self.shared
            .global_accumulator
            .lock()
            .merge_from(&self.local);
        self.local = PositionAccumulator::default();
    }
}

impl binseq::ParallelProcessor for GexDetectionProcessor {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self.scan_record(record.sseq(), record.xseq());
        Ok(())
    }
    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        self.flush();
        Ok(())
    }
    fn set_tid(&mut self, tid: usize) {
        self.tid = tid;
    }
    fn get_tid(&self) -> Option<usize> {
        Some(self.tid)
    }
}

impl<Rf: paraseq::Record> paraseq::prelude::PairedParallelProcessor<Rf> for GexDetectionProcessor {
    fn process_record_pair(&mut self, record1: Rf, record2: Rf) -> paraseq::Result<()> {
        self.scan_record(record1.seq().as_ref(), record2.seq().as_ref());
        Ok(())
    }
    fn on_batch_complete(&mut self) -> paraseq::Result<()> {
        self.flush();
        Ok(())
    }
    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }
    fn get_thread_id(&self) -> usize {
        self.tid
    }
}

// ---------------------------------------------------------------------------
// CRISPR detection processor
// ---------------------------------------------------------------------------

/// Shared state for CRISPR detection processors.
struct CrisprSharedState {
    whitelist: WhitelistMapper<Unpositioned>,
    crispr: CrisprMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    global_accumulator: Mutex<PositionAccumulator>,
    counter: AtomicUsize,
}

/// Processor for CRISPR geometry detection.
struct CrisprDetectionProcessor {
    shared: Arc<CrisprSharedState>,
    local: PositionAccumulator,
    tid: usize,
}

impl Clone for CrisprDetectionProcessor {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            local: PositionAccumulator::default(),
            tid: self.tid,
        }
    }
}

impl CrisprDetectionProcessor {
    fn scan_record(&mut self, r1_seq: &[u8], r2_seq: &[u8]) {
        self.local.total_reads += 1;

        for (seq, mate) in [(r1_seq, ReadMate::R1), (r2_seq, ReadMate::R2)] {
            for pos in self.shared.whitelist.scan_positions(seq) {
                self.local.record_position(Component::Barcode, mate, pos);
            }
            for pos in self.shared.crispr.scan_anchor_positions(seq) {
                self.local.record_position(Component::Anchor, mate, pos);
            }
            for pos in self.shared.crispr.scan_protospacer_positions(seq) {
                self.local
                    .record_position(Component::Protospacer, mate, pos);
            }
            if let Some(ref probe) = self.shared.probe {
                for pos in probe.scan_positions(seq) {
                    self.local.record_position(Component::Probe, mate, pos);
                }
            }
        }
    }

    fn flush(&mut self) {
        self.shared
            .global_accumulator
            .lock()
            .merge_from(&self.local);
        self.local = PositionAccumulator::default();
    }
}

impl binseq::ParallelProcessor for CrisprDetectionProcessor {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self.scan_record(record.sseq(), record.xseq());
        Ok(())
    }
    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        self.flush();
        Ok(())
    }
    fn set_tid(&mut self, tid: usize) {
        self.tid = tid;
    }
    fn get_tid(&self) -> Option<usize> {
        Some(self.tid)
    }
}

impl<Rf: paraseq::Record> paraseq::prelude::PairedParallelProcessor<Rf>
    for CrisprDetectionProcessor
{
    fn process_record_pair(&mut self, record1: Rf, record2: Rf) -> paraseq::Result<()> {
        self.scan_record(record1.seq().as_ref(), record2.seq().as_ref());
        Ok(())
    }
    fn on_batch_complete(&mut self) -> paraseq::Result<()> {
        self.flush();
        Ok(())
    }
    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }
    fn get_thread_id(&self) -> usize {
        self.tid
    }
}

// ---------------------------------------------------------------------------
// Read sampling
// ---------------------------------------------------------------------------

/// Sample reads for GEX detection, returning per-file accumulators.
///
/// Moves the mappers into shared state. Returns one `(label, accumulator)`
/// per input lane. For BINSEQ each file is one lane; for FASTX each
/// consecutive pair of files is one lane.
fn sample_gex_reads(
    whitelist: WhitelistMapper<Unpositioned>,
    gex: GexMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    config: &DetectionConfig,
) -> Result<Vec<(String, PositionAccumulator)>> {
    let shared = Arc::new(GexSharedState {
        whitelist,
        gex,
        probe,
        global_accumulator: Mutex::new(PositionAccumulator::default()),
        counter: AtomicUsize::new(0),
        limit: config.num_reads,
    });

    let mut results = Vec::new();

    if input.is_binseq() {
        for (i, path) in input.inputs.iter().enumerate() {
            // Reset for this file.
            shared.counter.store(0, Ordering::Relaxed);
            *shared.global_accumulator.lock() = PositionAccumulator::default();

            let proc = GexDetectionProcessor {
                shared: Arc::clone(&shared),
                local: PositionAccumulator::default(),
                tid: 0,
            };

            let reader = BinseqReader::new(path)?;
            let n = reader.num_records()?.min(config.num_reads);
            if n > 0 {
                reader.process_parallel_range(proc, config.num_threads, 0..n)?;
            }

            let label = format!("lane{}: {}", i + 1, path);
            results.push((label, shared.global_accumulator.lock().clone()));
        }
    } else {
        for (i, chunk) in input.inputs.chunks(2).enumerate() {
            // Reset for this lane.
            shared.counter.store(0, Ordering::Relaxed);
            *shared.global_accumulator.lock() = PositionAccumulator::default();

            let mut proc = GexDetectionProcessor {
                shared: Arc::clone(&shared),
                local: PositionAccumulator::default(),
                tid: 0,
            };

            let lane_input = MultiPairedInput {
                inputs: chunk.to_vec(),
            };
            let collection = lane_input.to_paraseq_collection()?;
            collection.process_parallel_paired_range(
                &mut proc,
                config.num_threads,
                None,
                0..config.num_reads,
            )?;
            proc.flush();

            let label = format!("lane{}: {}", i + 1, chunk.join(" + "));
            results.push((label, shared.global_accumulator.lock().clone()));
        }
    }

    Ok(results)
}

/// Sample reads for CRISPR detection, returning per-file accumulators.
///
/// Moves the mappers into shared state. Returns one `(label, accumulator)`
/// per input lane.
fn sample_crispr_reads(
    whitelist: WhitelistMapper<Unpositioned>,
    crispr: CrisprMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    config: &DetectionConfig,
) -> Result<Vec<(String, PositionAccumulator)>> {
    let shared = Arc::new(CrisprSharedState {
        whitelist,
        crispr,
        probe,
        global_accumulator: Mutex::new(PositionAccumulator::default()),
        counter: AtomicUsize::new(0),
    });

    let mut results = Vec::new();

    if input.is_binseq() {
        for (i, path) in input.inputs.iter().enumerate() {
            shared.counter.store(0, Ordering::Relaxed);
            *shared.global_accumulator.lock() = PositionAccumulator::default();

            let proc = CrisprDetectionProcessor {
                shared: Arc::clone(&shared),
                local: PositionAccumulator::default(),
                tid: 0,
            };

            let reader = BinseqReader::new(path)?;
            let n = reader.num_records()?.min(config.num_reads);
            if n > 0 {
                reader.process_parallel_range(proc, config.num_threads, 0..n)?;
            }

            let label = format!("lane{}: {}", i + 1, path);
            results.push((label, shared.global_accumulator.lock().clone()));
        }
    } else {
        for (i, chunk) in input.inputs.chunks(2).enumerate() {
            shared.counter.store(0, Ordering::Relaxed);
            *shared.global_accumulator.lock() = PositionAccumulator::default();

            let mut proc = CrisprDetectionProcessor {
                shared: Arc::clone(&shared),
                local: PositionAccumulator::default(),
                tid: 0,
            };

            let lane_input = MultiPairedInput {
                inputs: chunk.to_vec(),
            };
            let collection = lane_input.to_paraseq_collection()?;
            collection.process_parallel_paired_range(
                &mut proc,
                config.num_threads,
                None,
                0..config.num_reads,
            )?;
            proc.flush();

            let label = format!("lane{}: {}", i + 1, chunk.join(" + "));
            results.push((label, shared.global_accumulator.lock().clone()));
        }
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Geometry inference
// ---------------------------------------------------------------------------

/// Assigned component: the best (mate, position) for a component.
struct ComponentAssignment {
    component: Component,
    mate: ReadMate,
    position: usize,
    seq_len: Option<usize>,
    count: usize,
    top_positions: Vec<(ReadMate, usize, usize)>,
}

/// Find the best (mate, position) for each component, returning top candidates.
fn find_best_positions(
    accumulator: &PositionAccumulator,
    components: &[Component],
) -> Vec<ComponentAssignment> {
    let mut assignments = Vec::new();

    for &comp in components {
        let mut positions: Vec<(ReadMate, usize, usize)> = accumulator
            .counts
            .iter()
            .filter(|((c, _, _), _)| *c == comp)
            .map(|((_, mate, pos), &count)| (*mate, *pos, count))
            .collect();

        positions.sort_by(|a, b| b.2.cmp(&a.2));

        let top_positions: Vec<_> = positions.iter().take(5).copied().collect();

        if let Some(&(mate, pos, count)) = positions.first() {
            assignments.push(ComponentAssignment {
                component: comp,
                mate,
                position: pos,
                seq_len: None, // filled in later
                count,
                top_positions,
            });
        }
    }

    assignments
}

/// Check if two ranges overlap on the same mate.
fn ranges_overlap(
    mate_a: ReadMate,
    pos_a: usize,
    len_a: usize,
    mate_b: ReadMate,
    pos_b: usize,
    len_b: usize,
) -> bool {
    if mate_a != mate_b {
        return false;
    }
    let end_a = pos_a + len_a;
    let end_b = pos_b + len_b;
    pos_a < end_b && pos_b < end_a
}

/// Resolve overlapping assignments. Higher count wins; loser falls back to next-best.
fn resolve_overlaps(assignments: &mut [ComponentAssignment]) -> Result<()> {
    let max_iterations = 20;
    for _ in 0..max_iterations {
        let mut conflict = None;

        'outer: for i in 0..assignments.len() {
            for j in (i + 1)..assignments.len() {
                let len_i = assignments[i].seq_len.unwrap_or(1);
                let len_j = assignments[j].seq_len.unwrap_or(1);

                if ranges_overlap(
                    assignments[i].mate,
                    assignments[i].position,
                    len_i,
                    assignments[j].mate,
                    assignments[j].position,
                    len_j,
                ) {
                    let loser = if assignments[i].count >= assignments[j].count {
                        j
                    } else {
                        i
                    };
                    conflict = Some(loser);
                    break 'outer;
                }
            }
        }

        let Some(loser_idx) = conflict else {
            return Ok(());
        };

        let current_mate = assignments[loser_idx].mate;
        let current_pos = assignments[loser_idx].position;
        let top = assignments[loser_idx].top_positions.clone();

        let mut found_alt = false;
        for &(alt_mate, alt_pos, alt_count) in &top {
            if alt_mate == current_mate && alt_pos == current_pos {
                continue;
            }

            let alt_len = assignments[loser_idx].seq_len.unwrap_or(1);
            let conflicts = assignments.iter().enumerate().any(|(k, a)| {
                k != loser_idx
                    && ranges_overlap(
                        alt_mate,
                        alt_pos,
                        alt_len,
                        a.mate,
                        a.position,
                        a.seq_len.unwrap_or(1),
                    )
            });

            if !conflicts {
                assignments[loser_idx].mate = alt_mate;
                assignments[loser_idx].position = alt_pos;
                assignments[loser_idx].count = alt_count;
                found_alt = true;
                break;
            }
        }

        if !found_alt {
            bail!(
                "cannot find non-overlapping position for [{}]. \
                 Best position ({:?}, {}) overlaps with another component.",
                assignments[loser_idx].component,
                current_mate,
                current_pos,
            );
        }
    }

    bail!("could not resolve overlapping component positions after {max_iterations} iterations");
}

/// Infer geometry from accumulated position data.
fn infer_geometry(
    accumulator: &PositionAccumulator,
    mode: DetectionMode,
    has_probe: bool,
    component_seq_lens: &HashMap<Component, Option<usize>>,
    config: &DetectionConfig,
) -> Result<DetectionResult> {
    let total_reads = accumulator.total_reads;

    if total_reads == 0 {
        bail!("0 reads were sampled during geometry detection. Is the input file empty?");
    }

    if total_reads < 10_000 {
        #[allow(clippy::cast_sign_loss)]
        // total_reads * remap_min_proportion is non-negative; cast cannot lose sign.
        // cast_possible_truncation is globally allowed in workspace Cargo.toml.
        let min_hits = ((total_reads as f64 * config.remap_min_proportion).ceil() as usize).max(1);
        warn!(
            "Only {total_reads} reads sampled for geometry detection \
             (min_hits={min_hits} at remap_min_proportion={:.2}); \
             confidence may be low. Increase --num-reads or raise --remap-min-proportion.",
            config.remap_min_proportion
        );
    }

    // Determine required components
    let mut components: Vec<Component> = vec![Component::Barcode];
    match mode {
        DetectionMode::Gex => components.push(Component::Gex),
        DetectionMode::Crispr => {
            components.push(Component::Anchor);
            components.push(Component::Protospacer);
        }
    }
    if has_probe {
        components.push(Component::Probe);
    }

    let mut assignments = find_best_positions(accumulator, &components);

    // Fill in seq_lens
    for assignment in &mut assignments {
        assignment.seq_len = component_seq_lens
            .get(&assignment.component)
            .copied()
            .flatten();
    }

    resolve_overlaps(&mut assignments)?;

    // Validate proportions
    for assignment in &assignments {
        let proportion = assignment.count as f64 / total_reads as f64;
        if proportion < config.min_proportion {
            bail!(
                "component [{}] has match proportion {:.4} ({}/{} reads), \
                 below threshold {:.2}. Geometry detection failed.\n\
                 Provide --geometry or --preset manually.",
                assignment.component,
                proportion,
                assignment.count,
                total_reads,
                config.min_proportion,
            );
        }
    }

    let remap_window = estimate_remap_window(
        accumulator,
        &components,
        total_reads,
        config.remap_min_proportion,
    );

    // Build evidence (windowed fields use the just-computed `remap_window`).
    let evidence: Vec<ComponentEvidence> = assignments
        .iter()
        .map(|a| {
            let windowed_count =
                accumulator.windowed_count(a.component, a.mate, a.position, remap_window);
            ComponentEvidence {
                component: a.component,
                mate: a.mate,
                position: a.position,
                seq_len: a.seq_len,
                match_count: a.count,
                match_proportion: a.count as f64 / total_reads as f64,
                windowed_match_count: windowed_count,
                windowed_match_proportion: windowed_count as f64 / total_reads as f64,
                top_positions: a.top_positions.clone(),
            }
        })
        .collect();

    // Insert UMI: same mate as barcode, right after barcode
    let barcode = assignments
        .iter()
        .find(|a| a.component == Component::Barcode)
        .expect("barcode assignment must exist");
    let barcode_seq_len = barcode.seq_len.expect("barcode seq_len must be known");
    let umi_mate = barcode.mate;
    let umi_pos = barcode.position + barcode_seq_len;
    let umi_len: usize = FLEX_UMI_LENGTH;

    // Build placement list for geometry construction
    let mut placements: Vec<(Component, ReadMate, usize, Option<usize>)> = assignments
        .iter()
        .map(|a| (a.component, a.mate, a.position, a.seq_len))
        .collect();
    placements.push((Component::Umi, umi_mate, umi_pos, Some(umi_len)));

    // Build geometry
    let r1 = build_read_regions(&placements, ReadMate::R1);
    let r2 = build_read_regions(&placements, ReadMate::R2);
    let geometry = Geometry { r1, r2 };
    let geometry_string = format_geometry_string(&geometry);

    Ok(DetectionResult {
        geometry,
        geometry_string,
        remap_window,
        evidence,
        total_reads_sampled: total_reads,
        per_file_results: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Geometry building helpers
// ---------------------------------------------------------------------------

/// Build ordered regions for a single read mate.
fn build_read_regions(
    placements: &[(Component, ReadMate, usize, Option<usize>)],
    mate: ReadMate,
) -> Read {
    let mut mate_placements: Vec<_> = placements
        .iter()
        .filter(|(_, m, _, _)| *m == mate)
        .copied()
        .collect();
    mate_placements.sort_by_key(|(_, _, pos, _)| *pos);

    let mut regions = Vec::new();
    let mut cursor = 0usize;
    let mut prev_variable = false;

    for (component, _, pos, len) in &mate_placements {
        // Only insert a skip if the previous component had a known length.
        // Variable-length components (e.g. anchor) are assumed to fill the
        // gap to the next component, so no skip is emitted after them.
        if *pos > cursor && !prev_variable {
            regions.push(Region::Skip {
                length: pos - cursor,
            });
        }

        let length = if component.requires_length() {
            *len
        } else {
            None
        };
        regions.push(Region::Component {
            kind: *component,
            length,
        });

        if let Some(l) = len {
            cursor = pos + l;
            prev_variable = false;
        } else {
            // Variable-length: cursor stays at pos, mark so next iteration
            // skips gap insertion.
            cursor = *pos;
            prev_variable = true;
        }
    }

    Read { regions }
}

/// Format a Geometry into a human-readable string.
fn format_geometry_string(geometry: &Geometry) -> String {
    let r1 = format_read_string(&geometry.r1);
    let r2 = format_read_string(&geometry.r2);
    format!("{r1} | {r2}")
}

fn format_read_string(read: &Read) -> String {
    read.regions
        .iter()
        .map(|r| match r {
            Region::Skip { length } => format!("[:{length}]"),
            Region::Component { kind, length } => {
                if let Some(len) = length {
                    format!("[{kind}:{len}]")
                } else {
                    format!("[{kind}]")
                }
            }
        })
        .collect::<String>()
}

// ---------------------------------------------------------------------------
// Remap window estimation
// ---------------------------------------------------------------------------

/// Minimum hit count for a position to be considered real (not noise) when
/// estimating the remap window.  Sequence hashes are exact-match, so for any
/// component >= 8 bp the expected random-match rate is negligible.
/// Default minimum proportion of sampled reads for remap window estimation.
#[cfg(test)]
const DEFAULT_REMAP_MIN_PROPORTION: f64 = 0.01;

/// Estimate the optimal remap window from position distributions.
///
/// Excludes `[barcode]` only.  Barcodes sit at position 0 by chemistry in
/// both Flex V1 and V2, so any apparent spread is artifactual (poly-N
/// bleed-through, adapter contamination) and should not inflate the
/// recommendation.
///
/// All other components -- including `[probe]` -- are included.  On some
/// Flex V2 libraries the `[:18]` spacer between `[gex]` and `[probe]`
/// drifts in length (template switching / RT artifacts), producing real
/// positional spread on `[probe]` that the recommendation must capture.
/// The contiguous-range walk + `min_hits = ceil(total_reads *
/// remap_min_proportion)` threshold remain self-protecting against
/// short-sequence false matches: isolated low-count positions are
/// filtered, and the walk stops at the first sub-threshold position so
/// disjoint false-match clusters cannot extend the window.
///
/// Uses a contiguous-range walk from the best position: starting at the
/// mode, walks outward in both directions, stopping when a position has
/// fewer than `min_hits` matches.  This captures smooth exponential
/// tails (e.g. Flex V2 anchor at positions 9-19, or V2 probe with
/// spacer drift) while excluding isolated outliers (e.g. a chimeric
/// read matching a protospacer 15 bp away from the main cluster).
fn estimate_remap_window(
    accumulator: &PositionAccumulator,
    components: &[Component],
    total_reads: usize,
    remap_min_proportion: f64,
) -> usize {
    #[allow(clippy::cast_sign_loss)]
    // total_reads * remap_min_proportion is non-negative; cast cannot lose sign.
    // cast_possible_truncation is globally allowed in workspace Cargo.toml.
    let min_hits = ((total_reads as f64 * remap_min_proportion).ceil() as usize).max(1);
    log::trace!(
        "remap_window: min_hits={min_hits} (proportion={remap_min_proportion}, reads={total_reads})"
    );

    let mut max_window = 0usize;

    for &comp in components {
        // Barcode positions are chemistry-fixed at 0; any apparent spread
        // is artifactual (poly-N bleed-through, adapter contamination).
        // Probe is intentionally NOT excluded: V2 libraries can have real
        // [:18] spacer drift, and the min_hits threshold + contiguous walk
        // already filter short-sequence false matches.
        if matches!(comp, Component::Barcode) {
            continue;
        }

        // Find the best (mate, position) for this component.
        let best_entry = accumulator
            .counts
            .iter()
            .filter(|((c, _, _), _)| *c == comp)
            .max_by_key(|&(_, count)| count);

        let Some((&(_, best_mate, best_pos), &best_count)) = best_entry else {
            continue;
        };

        if best_count == 0 {
            continue;
        }

        // Build a lookup of counts by position on the best mate.
        let pos_counts: HashMap<usize, usize> = accumulator
            .counts
            .iter()
            .filter(|((c, mate, _), _)| *c == comp && *mate == best_mate)
            .map(|((_, _, pos), count)| (*pos, *count))
            .collect();

        // Walk outward from best_pos, requiring contiguous significant
        // positions.  Stops at the first gap (position with < min_hits).
        let mut farthest_below = best_pos;
        {
            let mut pos = best_pos;
            while pos > 0 {
                pos -= 1;
                if pos_counts.get(&pos).copied().unwrap_or(0) >= min_hits {
                    farthest_below = pos;
                } else {
                    break;
                }
            }
        }

        let mut farthest_above = best_pos;
        {
            let mut pos = best_pos;
            loop {
                pos += 1;
                if pos_counts.get(&pos).copied().unwrap_or(0) >= min_hits {
                    farthest_above = pos;
                } else {
                    break;
                }
            }
        }

        let window = (best_pos - farthest_below).max(farthest_above - best_pos);

        if window > 0 {
            log::trace!(
                "remap_window: [{comp}] best=({best_mate:?}, {best_pos}, {best_count}) \
                 contiguous range={farthest_below}..={farthest_above} window={window}",
            );
        }

        max_window = max_window.max(window);
    }

    if max_window == 0 { 1 } else { max_window }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate that all per-file detection results agree on geometry and aggregate
/// into a single `DetectionResult` with the maximum remap window.
#[allow(clippy::too_many_lines)]
fn validate_and_aggregate(
    per_file: Vec<(String, PositionAccumulator, DetectionResult)>,
) -> Result<DetectionResult> {
    if per_file.is_empty() {
        bail!("validate_and_aggregate called with no detection results");
    }

    // Check geometry consistency.
    let first_geometry = &per_file[0].2.geometry_string;
    let mismatches: Vec<_> = per_file
        .iter()
        .filter(|(_, _, r)| r.geometry_string != *first_geometry)
        .collect();

    if !mismatches.is_empty() {
        use std::fmt::Write;
        let mut msg = String::from("Geometry mismatch across input files:\n");
        for (label, _, result) in &per_file {
            writeln!(msg, "  {label}: {}", result.geometry_string).unwrap();
        }
        write!(
            msg,
            "All input files must produce the same detected geometry."
        )
        .unwrap();
        bail!("{msg}");
    }

    // Aggregate: max remap window, sum reads, collect per-file results.
    let max_remap_window = per_file
        .iter()
        .map(|(_, _, r)| r.remap_window)
        .max()
        .unwrap();
    let total_reads: usize = per_file.iter().map(|(_, _, r)| r.total_reads_sampled).sum();

    // Merge per-file accumulators for the aggregated windowed re-walk.
    let mut aggregated_accumulator = PositionAccumulator::default();
    for (_, acc, _) in &per_file {
        aggregated_accumulator.merge_from(acc);
    }

    let per_file_results: Vec<PerFileResult> = per_file
        .iter()
        .map(|(label, _, r)| PerFileResult {
            label: label.clone(),
            geometry_string: r.geometry_string.clone(),
            remap_window: r.remap_window,
            evidence: r
                .evidence
                .iter()
                .map(|ev| ComponentEvidence {
                    component: ev.component,
                    mate: ev.mate,
                    position: ev.position,
                    seq_len: ev.seq_len,
                    match_count: ev.match_count,
                    match_proportion: ev.match_proportion,
                    windowed_match_count: ev.windowed_match_count,
                    windowed_match_proportion: ev.windowed_match_proportion,
                    top_positions: ev.top_positions.clone(),
                })
                .collect(),
            total_reads_sampled: r.total_reads_sampled,
        })
        .collect();

    // Aggregate evidence across all files: sum counts, recompute proportions,
    // and re-walk the merged accumulator at `max_remap_window` for windowed
    // counts (using file 0's best position as the canonical center, matching
    // the aggregated `geometry` source).
    let first_evidence = &per_file[0].2.evidence;
    let aggregated_evidence: Vec<ComponentEvidence> = first_evidence
        .iter()
        .enumerate()
        .map(|(i, first_ev)| {
            let total_count: usize = per_file
                .iter()
                .map(|(_, _, r)| r.evidence[i].match_count)
                .sum();
            let proportion = if total_reads > 0 {
                total_count as f64 / total_reads as f64
            } else {
                0.0
            };

            let windowed_count = aggregated_accumulator.windowed_count(
                first_ev.component,
                first_ev.mate,
                first_ev.position,
                max_remap_window,
            );
            let windowed_proportion = if total_reads > 0 {
                windowed_count as f64 / total_reads as f64
            } else {
                0.0
            };

            // Merge top_positions across files.
            let mut pos_counts: HashMap<(ReadMate, usize), usize> = HashMap::new();
            for (_, _, r) in &per_file {
                for &(mate, pos, count) in &r.evidence[i].top_positions {
                    *pos_counts.entry((mate, pos)).or_default() += count;
                }
            }
            let mut top_positions: Vec<(ReadMate, usize, usize)> = pos_counts
                .into_iter()
                .map(|((mate, pos), count)| (mate, pos, count))
                .collect();
            top_positions.sort_by(|a, b| b.2.cmp(&a.2));

            ComponentEvidence {
                component: first_ev.component,
                mate: first_ev.mate,
                position: first_ev.position,
                seq_len: first_ev.seq_len,
                match_count: total_count,
                match_proportion: proportion,
                windowed_match_count: windowed_count,
                windowed_match_proportion: windowed_proportion,
                top_positions,
            }
        })
        .collect();

    // Take the first file's geometry by consuming the vec.
    let (_, _, first) = per_file.into_iter().next().unwrap();

    Ok(DetectionResult {
        geometry: first.geometry,
        geometry_string: first.geometry_string,
        remap_window: max_remap_window,
        evidence: aggregated_evidence,
        total_reads_sampled: total_reads,
        per_file_results,
    })
}

/// Log a per-file detection result.
fn log_per_file_result(label: &str, result: &DetectionResult) {
    info!(
        "  [{}] geometry=`{}` remap_window={} reads={}",
        label, result.geometry_string, result.remap_window, result.total_reads_sampled
    );
    for ev in &result.evidence {
        info!(
            "    [{}] {:?} pos={} count={} proportion={:.4} windowed_count={} windowed_proportion={:.4}",
            ev.component,
            ev.mate,
            ev.position,
            ev.match_count,
            ev.match_proportion,
            ev.windowed_match_count,
            ev.windowed_match_proportion,
        );
    }
}

/// Log the aggregated detection result.
pub fn log_detection_result(result: &DetectionResult) {
    let num_files = result.per_file_results.len();
    info!("Detected geometry: `{}`", result.geometry_string);
    info!("Recommended --remap-window: {}", result.remap_window);
    if num_files > 1 {
        info!(
            "Detection sampled {} reads total ({} files)",
            result.total_reads_sampled, num_files
        );
    } else {
        info!(
            "Detection sampled {} reads total",
            result.total_reads_sampled
        );
    }
    for ev in &result.evidence {
        info!(
            "  [{}] {:?} pos={} count={} proportion={:.4} windowed_count={} windowed_proportion={:.4}",
            ev.component,
            ev.mate,
            ev.position,
            ev.match_count,
            ev.match_proportion,
            ev.windowed_match_count,
            ev.windowed_match_proportion,
        );
        log_top_alternatives(ev);
    }
}

/// Log top alternative positions for a component (up to 3).
fn log_top_alternatives(ev: &ComponentEvidence) {
    for &(mate, pos, count) in ev.top_positions.iter().skip(1).take(3) {
        info!("    alt: {mate:?} pos={pos} count={count}");
    }
}

/// Detect GEX geometry by sampling reads and scanning for component positions.
///
/// Samples each input file independently, validates that all files produce the
/// same geometry string, and returns the result with the maximum remap window.
///
/// The mappers are moved into the detection processor and consumed.
/// Callers should create fresh mappers for the actual mapping pipeline after
/// detection returns.
pub fn detect_gex_geometry(
    whitelist: WhitelistMapper<Unpositioned>,
    gex: GexMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    config: &DetectionConfig,
) -> Result<DetectionResult> {
    let num_lanes = if input.is_binseq() {
        input.inputs.len()
    } else {
        input.inputs.len() / 2
    };
    info!(
        "Detecting GEX geometry from {} reads per file ({} lane{})...",
        config.num_reads,
        num_lanes,
        if num_lanes == 1 { "" } else { "s" },
    );

    let mut component_seq_lens: HashMap<Component, Option<usize>> = HashMap::new();
    component_seq_lens.insert(Component::Barcode, Some(whitelist.seq_len()));
    component_seq_lens.insert(Component::Gex, Some(gex.seq_len()));
    if let Some(ref p) = probe {
        component_seq_lens.insert(Component::Probe, Some(p.seq_len()));
    }

    let has_probe = probe.is_some();
    let per_file_accumulators = sample_gex_reads(whitelist, gex, probe, input, config)?;

    let mut per_file_results = Vec::with_capacity(per_file_accumulators.len());
    for (label, accumulator) in per_file_accumulators {
        let result = infer_geometry(
            &accumulator,
            DetectionMode::Gex,
            has_probe,
            &component_seq_lens,
            config,
        )?;
        log_per_file_result(&label, &result);
        per_file_results.push((label, accumulator, result));
    }

    validate_and_aggregate(per_file_results)
}

/// Detect CRISPR geometry by sampling reads and scanning for component positions.
///
/// Samples each input file independently, validates that all files produce the
/// same geometry string, and returns the result with the maximum remap window.
///
/// The mappers are moved into the detection processor and consumed.
pub fn detect_crispr_geometry(
    whitelist: WhitelistMapper<Unpositioned>,
    crispr: CrisprMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    config: &DetectionConfig,
) -> Result<DetectionResult> {
    let num_lanes = if input.is_binseq() {
        input.inputs.len()
    } else {
        input.inputs.len() / 2
    };
    info!(
        "Detecting CRISPR geometry from {} reads per file ({} lane{})...",
        config.num_reads,
        num_lanes,
        if num_lanes == 1 { "" } else { "s" },
    );

    let mut component_seq_lens: HashMap<Component, Option<usize>> = HashMap::new();
    component_seq_lens.insert(Component::Barcode, Some(whitelist.seq_len()));
    component_seq_lens.insert(Component::Anchor, crispr.anchor_len());
    component_seq_lens.insert(Component::Protospacer, Some(crispr.protospacer_len()));
    if let Some(ref p) = probe {
        component_seq_lens.insert(Component::Probe, Some(p.seq_len()));
    }

    let has_probe = probe.is_some();
    let per_file_accumulators = sample_crispr_reads(whitelist, crispr, probe, input, config)?;

    let mut per_file_results = Vec::with_capacity(per_file_accumulators.len());
    for (label, accumulator) in per_file_accumulators {
        let result = infer_geometry(
            &accumulator,
            DetectionMode::Crispr,
            has_probe,
            &component_seq_lens,
            config,
        )?;
        log_per_file_result(&label, &result);
        per_file_results.push((label, accumulator, result));
    }

    validate_and_aggregate(per_file_results)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    /// Helper: build a `PositionAccumulator` with specified entries.
    fn build_accumulator(
        entries: &[(Component, ReadMate, usize, usize)],
        total_reads: usize,
    ) -> PositionAccumulator {
        let mut acc = PositionAccumulator::default();
        acc.total_reads = total_reads;
        for &(comp, mate, pos, count) in entries {
            acc.counts.insert((comp, mate, pos), count);
        }
        acc
    }

    // -------------------------------------------------------------------
    // infer_geometry tests
    // -------------------------------------------------------------------

    #[test]
    fn test_infer_gex_geometry_basic() {
        // Simulate: barcode at R1:0, gex at R2:0
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 0, 4000),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap();

        // Barcode at R1:0, UMI at R1:16, Gex at R2:0
        assert_eq!(result.geometry_string, "[barcode][umi:12] | [gex]");
        assert_eq!(result.total_reads_sampled, 10000);
        assert_eq!(result.evidence.len(), 2); // barcode + gex
    }

    #[test]
    fn test_infer_gex_geometry_with_probe() {
        // Simulate: barcode at R1:0, gex at R2:0, probe at R2:68
        // (gex=50bp, gap=18, probe at 68)
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 0, 4000),
                (Component::Probe, ReadMate::R2, 68, 3500),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));
        seq_lens.insert(Component::Probe, Some(8));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, true, &seq_lens, &config).unwrap();

        assert_eq!(
            result.geometry_string,
            "[barcode][umi:12] | [gex][:18][probe]"
        );
        assert_eq!(result.evidence.len(), 3); // barcode + gex + probe
    }

    #[test]
    fn test_infer_crispr_geometry_basic() {
        // Simulate: barcode at R1:0, anchor at R2:0, protospacer at R2:33
        // anchor is variable-length (None)
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Anchor, ReadMate::R2, 0, 4000),
                (Component::Protospacer, ReadMate::R2, 33, 3500),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Anchor, None); // variable-length
        seq_lens.insert(Component::Protospacer, Some(20));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result =
            infer_geometry(&acc, DetectionMode::Crispr, false, &seq_lens, &config).unwrap();

        // anchor is variable-length; it fills the gap to protospacer (no skip)
        assert!(result.geometry_string.contains("[barcode][umi:12]"));
        assert!(
            result.geometry_string.contains("[anchor][protospacer]"),
            "expected [anchor][protospacer] without skip, got: {}",
            result.geometry_string,
        );
    }

    // -------------------------------------------------------------------
    // Skip region insertion test
    // -------------------------------------------------------------------

    #[test]
    fn test_skip_region_inserted_for_gap() {
        // barcode at R1:0 (16bp), gex at R2:5 (50bp) -- gap of 5 at R2 start
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 5, 4000),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap();

        assert_eq!(result.geometry_string, "[barcode][umi:12] | [:5][gex]");
    }

    // -------------------------------------------------------------------
    // UMI insertion test
    // -------------------------------------------------------------------

    #[test]
    fn test_umi_placed_after_barcode() {
        // Barcode at R1:3, 16bp => UMI should be at R1:19
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 3, 5000),
                (Component::Gex, ReadMate::R2, 0, 4000),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap();

        // Should have skip:3, barcode, umi:12 on R1
        assert_eq!(result.geometry_string, "[:3][barcode][umi:12] | [gex]");
    }

    // -------------------------------------------------------------------
    // Overlap detection
    // -------------------------------------------------------------------

    #[test]
    fn test_ranges_overlap_same_mate() {
        assert!(ranges_overlap(ReadMate::R1, 0, 16, ReadMate::R1, 10, 12));
        assert!(!ranges_overlap(ReadMate::R1, 0, 16, ReadMate::R1, 16, 12));
        assert!(!ranges_overlap(ReadMate::R1, 0, 16, ReadMate::R2, 10, 12));
    }

    #[test]
    fn test_overlap_resolution_falls_back() {
        // Two components claim overlapping positions on R2.
        // barcode at R1:0 (always non-overlapping).
        // gex at R2:0 (count=4000), probe at R2:5 (count=3000, overlaps gex 0..50)
        // probe also has an alternative at R2:68 (count=2500) which doesn't overlap.
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 0, 4000),
                (Component::Probe, ReadMate::R2, 5, 3000),
                (Component::Probe, ReadMate::R2, 68, 2500),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));
        seq_lens.insert(Component::Probe, Some(8));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, true, &seq_lens, &config).unwrap();

        // gex wins R2:0, probe should fall back to R2:68
        let probe_ev = result
            .evidence
            .iter()
            .find(|e| e.component == Component::Probe)
            .unwrap();
        assert_eq!(probe_ev.position, 68);
        assert_eq!(probe_ev.mate, ReadMate::R2);
    }

    // -------------------------------------------------------------------
    // Remap window estimation
    // -------------------------------------------------------------------

    #[test]
    fn test_remap_window_tight_distribution() {
        // All matches at a single position -> default window 1.
        let acc = build_accumulator(&[(Component::Gex, ReadMate::R2, 0, 5000)], 10000);
        let window =
            estimate_remap_window(&acc, &[Component::Gex], 10000, DEFAULT_REMAP_MIN_PROPORTION);
        assert_eq!(window, 1);
    }

    #[test]
    fn test_remap_window_contiguous_spread() {
        // Anchor positions 0-2 contiguous (all >= 3), gap at 3, isolated at 4.
        // Contiguous walk from 0: up → 1 → 2 → 3 (0 hits, stop).
        // Window = 2 (not 4, because pos 4 is non-contiguous).
        let acc = build_accumulator(
            &[
                (Component::Anchor, ReadMate::R2, 0, 5000),
                (Component::Anchor, ReadMate::R2, 1, 4000),
                (Component::Anchor, ReadMate::R2, 2, 3000),
                (Component::Anchor, ReadMate::R2, 4, 1000),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Anchor],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 2);
    }

    #[test]
    fn test_remap_window_isolated_outlier_ignored() {
        // Main cluster at 56-59 (contiguous), isolated outlier at 44.
        // The gap at 55 stops the contiguous walk -> pos 44 is excluded.
        let acc = build_accumulator(
            &[
                (Component::Protospacer, ReadMate::R2, 44, 5),
                (Component::Protospacer, ReadMate::R2, 56, 200),
                (Component::Protospacer, ReadMate::R2, 57, 100),
                (Component::Protospacer, ReadMate::R2, 58, 300),
                (Component::Protospacer, ReadMate::R2, 59, 5000),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Protospacer],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 3); // 59 - 56 = 3, pos 44 excluded
    }

    #[test]
    fn test_remap_window_noise_below_min_hits_ignored() {
        // Main at pos 0 (5000), noise at pos 1 (2, below 1% threshold).
        // Walk up stops immediately. Window = default 1.
        let acc = build_accumulator(
            &[
                (Component::Gex, ReadMate::R2, 0, 5000),
                (Component::Gex, ReadMate::R2, 1, 2),
            ],
            10000,
        );
        let window =
            estimate_remap_window(&acc, &[Component::Gex], 10000, DEFAULT_REMAP_MIN_PROPORTION);
        assert_eq!(window, 1);
    }

    #[test]
    fn test_remap_window_barcode_excluded() {
        // Barcode spread is excluded from remap window estimation
        // (barcode positions are chemistry-fixed, apparent spread is noise).
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Barcode, ReadMate::R1, 1, 4000),
                (Component::Barcode, ReadMate::R1, 2, 3000),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Barcode],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 1); // barcode is skipped, default returned
    }

    #[test]
    fn test_remap_window_probe_variable_positions() {
        // V2 GEX with variable [:18] spacer drift: probe at 60..=72, mode 66.
        // All counts >= 100 (min_hits = 1% of 10000). Window = max(66-60, 72-66) = 6.
        let acc = build_accumulator(
            &[
                (Component::Probe, ReadMate::R2, 60, 100),
                (Component::Probe, ReadMate::R2, 61, 150),
                (Component::Probe, ReadMate::R2, 62, 300),
                (Component::Probe, ReadMate::R2, 63, 600),
                (Component::Probe, ReadMate::R2, 64, 1200),
                (Component::Probe, ReadMate::R2, 65, 1800),
                (Component::Probe, ReadMate::R2, 66, 3000),
                (Component::Probe, ReadMate::R2, 67, 1800),
                (Component::Probe, ReadMate::R2, 68, 1200),
                (Component::Probe, ReadMate::R2, 69, 600),
                (Component::Probe, ReadMate::R2, 70, 300),
                (Component::Probe, ReadMate::R2, 71, 150),
                (Component::Probe, ReadMate::R2, 72, 100),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Probe],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 6);
    }

    #[test]
    fn test_remap_window_probe_noise_at_adjacent_position_ignored() {
        // Mode at 68 (8000), adjacent 67 (7000), adjacent outlier 69 (2 < 100 min_hits).
        // Walk down: 67 above, 66 absent -> stop. Walk up: 69 below threshold -> stop.
        // farthest_below = 67, farthest_above = 68.
        // window = max(best_pos - farthest_below, farthest_above - best_pos)
        //        = max(68 - 67, 68 - 68) = max(1, 0) = 1.
        // Exercises the threshold filter, not gap-stopping.
        let acc = build_accumulator(
            &[
                (Component::Probe, ReadMate::R2, 67, 7000),
                (Component::Probe, ReadMate::R2, 68, 8000),
                (Component::Probe, ReadMate::R2, 69, 2),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Probe],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 1);
    }

    #[test]
    fn test_remap_window_probe_two_clusters_separated_by_gap() {
        // True cluster at 66..=70 (mode 68); false-match cluster at 60..=63 (each 500);
        // gap at 64..=65 halts the walk. False cluster is above min_hits but unreachable.
        // Window = max(68 - 66, 70 - 68) = 2.
        let acc = build_accumulator(
            &[
                (Component::Probe, ReadMate::R2, 60, 500),
                (Component::Probe, ReadMate::R2, 61, 500),
                (Component::Probe, ReadMate::R2, 62, 500),
                (Component::Probe, ReadMate::R2, 63, 500),
                (Component::Probe, ReadMate::R2, 66, 1500),
                (Component::Probe, ReadMate::R2, 67, 2000),
                (Component::Probe, ReadMate::R2, 68, 3000),
                (Component::Probe, ReadMate::R2, 69, 2000),
                (Component::Probe, ReadMate::R2, 70, 1500),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Probe],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 2);
    }

    #[test]
    fn test_remap_window_probe_wider_than_gex() {
        // Joint-component call: gex tight (window=1), probe wide (window=6).
        // estimate_remap_window must return the max across both -> 6.
        let acc = build_accumulator(
            &[
                // gex: tight 3-position cluster.
                (Component::Gex, ReadMate::R2, 78, 1000),
                (Component::Gex, ReadMate::R2, 79, 5000),
                (Component::Gex, ReadMate::R2, 80, 1000),
                // probe: wide 13-position cluster (same as variable_positions test).
                (Component::Probe, ReadMate::R2, 60, 100),
                (Component::Probe, ReadMate::R2, 61, 150),
                (Component::Probe, ReadMate::R2, 62, 300),
                (Component::Probe, ReadMate::R2, 63, 600),
                (Component::Probe, ReadMate::R2, 64, 1200),
                (Component::Probe, ReadMate::R2, 65, 1800),
                (Component::Probe, ReadMate::R2, 66, 3000),
                (Component::Probe, ReadMate::R2, 67, 1800),
                (Component::Probe, ReadMate::R2, 68, 1200),
                (Component::Probe, ReadMate::R2, 69, 600),
                (Component::Probe, ReadMate::R2, 70, 300),
                (Component::Probe, ReadMate::R2, 71, 150),
                (Component::Probe, ReadMate::R2, 72, 100),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Gex, Component::Probe],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 6);
    }

    #[test]
    fn test_remap_window_variable_anchor_positions() {
        // Simulates Flex V2 CRISPR: anchor at positions 9-19, mode at 14.
        // All positions are contiguous with >= 1% of 10000 = 100 min_hits.
        let acc = build_accumulator(
            &[
                (Component::Anchor, ReadMate::R2, 9, 100),
                (Component::Anchor, ReadMate::R2, 10, 150),
                (Component::Anchor, ReadMate::R2, 11, 300),
                (Component::Anchor, ReadMate::R2, 12, 600),
                (Component::Anchor, ReadMate::R2, 13, 1500),
                (Component::Anchor, ReadMate::R2, 14, 8000),
                (Component::Anchor, ReadMate::R2, 15, 1500),
                (Component::Anchor, ReadMate::R2, 16, 600),
                (Component::Anchor, ReadMate::R2, 17, 300),
                (Component::Anchor, ReadMate::R2, 18, 150),
                (Component::Anchor, ReadMate::R2, 19, 100),
            ],
            10000,
        );
        let window = estimate_remap_window(
            &acc,
            &[Component::Anchor],
            10000,
            DEFAULT_REMAP_MIN_PROPORTION,
        );
        assert_eq!(window, 5); // |14-9| = 5
    }

    // -------------------------------------------------------------------
    // Validation
    // -------------------------------------------------------------------

    #[test]
    fn test_validation_below_threshold_fails() {
        // barcode proportion = 500/10000 = 0.05, below threshold 0.10
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 500),
                (Component::Gex, ReadMate::R2, 0, 4000),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let err = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("[barcode]"), "error should name the component");
        assert!(
            msg.contains("below threshold"),
            "error should mention threshold"
        );
    }

    #[test]
    fn test_validation_above_threshold_succeeds() {
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 2000),
                (Component::Gex, ReadMate::R2, 0, 1500),
            ],
            10000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_zero_reads_fails() {
        let acc = PositionAccumulator::default(); // 0 total_reads
        let seq_lens = HashMap::new();

        let config = DetectionConfig {
            num_reads: 10000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let err = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("0 reads"), "error should mention 0 reads");
    }

    // -------------------------------------------------------------------
    // build_read_regions tests
    // -------------------------------------------------------------------

    #[test]
    fn test_build_read_regions_empty_mate() {
        let placements = vec![(Component::Gex, ReadMate::R2, 0, Some(50))];
        let r1 = build_read_regions(&placements, ReadMate::R1);
        assert!(r1.regions.is_empty());
    }

    #[test]
    fn test_build_read_regions_with_gap() {
        let placements = vec![
            (Component::Barcode, ReadMate::R1, 0, Some(16)),
            (Component::Umi, ReadMate::R1, 16, Some(12)),
            (Component::Gex, ReadMate::R2, 5, Some(50)),
        ];
        let r2 = build_read_regions(&placements, ReadMate::R2);
        assert_eq!(r2.regions.len(), 2);
        assert!(matches!(r2.regions[0], Region::Skip { length: 5 }));
        assert!(matches!(
            r2.regions[1],
            Region::Component {
                kind: Component::Gex,
                length: None
            }
        ));
    }

    // -------------------------------------------------------------------
    // format_geometry_string tests
    // -------------------------------------------------------------------

    #[test]
    fn test_format_geometry_string_roundtrip() {
        let geometry = Geometry {
            r1: Read {
                regions: vec![
                    Region::Component {
                        kind: Component::Barcode,
                        length: None,
                    },
                    Region::Component {
                        kind: Component::Umi,
                        length: Some(12),
                    },
                ],
            },
            r2: Read {
                regions: vec![Region::Component {
                    kind: Component::Gex,
                    length: None,
                }],
            },
        };
        assert_eq!(
            format_geometry_string(&geometry),
            "[barcode][umi:12] | [gex]"
        );
    }

    // -------------------------------------------------------------------
    // PositionAccumulator tests
    // -------------------------------------------------------------------

    #[test]
    fn test_accumulator_merge() {
        let mut a = PositionAccumulator::default();
        a.total_reads = 100;
        a.record_position(Component::Barcode, ReadMate::R1, 0);
        a.record_position(Component::Barcode, ReadMate::R1, 0);

        let mut b = PositionAccumulator::default();
        b.total_reads = 50;
        b.record_position(Component::Barcode, ReadMate::R1, 0);
        b.record_position(Component::Barcode, ReadMate::R1, 5);

        a.merge_from(&b);

        assert_eq!(a.total_reads, 150);
        assert_eq!(a.counts[&(Component::Barcode, ReadMate::R1, 0)], 3);
        assert_eq!(a.counts[&(Component::Barcode, ReadMate::R1, 5)], 1);
    }

    // -------------------------------------------------------------------
    // validate_and_aggregate tests
    // -------------------------------------------------------------------

    /// Helper: build a minimal `DetectionResult` with the given geometry string
    /// and remap window.
    fn build_detection_result(
        geometry_string: &str,
        remap_window: usize,
        reads: usize,
    ) -> DetectionResult {
        DetectionResult {
            geometry: Geometry {
                r1: Read { regions: vec![] },
                r2: Read { regions: vec![] },
            },
            geometry_string: geometry_string.to_string(),
            remap_window,
            evidence: vec![ComponentEvidence {
                component: Component::Barcode,
                mate: ReadMate::R1,
                position: 0,
                seq_len: Some(16),
                match_count: reads / 2,
                match_proportion: 0.5,
                windowed_match_count: reads / 2,
                windowed_match_proportion: 0.5,
                top_positions: vec![(ReadMate::R1, 0, reads / 2)],
            }],
            total_reads_sampled: reads,
            per_file_results: Vec::new(),
        }
    }

    #[test]
    fn test_validate_and_aggregate_single_file() {
        let r = build_detection_result("[barcode][umi:12] | [gex]", 3, 10000);
        let result = validate_and_aggregate(vec![(
            "file1.cbq".to_string(),
            PositionAccumulator::default(),
            r,
        )])
        .unwrap();

        assert_eq!(result.geometry_string, "[barcode][umi:12] | [gex]");
        assert_eq!(result.remap_window, 3);
        assert_eq!(result.total_reads_sampled, 10000);
        assert_eq!(result.per_file_results.len(), 1);
        assert_eq!(result.per_file_results[0].label, "file1.cbq");
    }

    #[test]
    fn test_validate_and_aggregate_matching_geometries_max_remap() {
        let r1 = build_detection_result("[barcode][umi:12] | [gex]", 2, 10000);
        let r2 = build_detection_result("[barcode][umi:12] | [gex]", 5, 8000);
        let result = validate_and_aggregate(vec![
            ("lane1.cbq".to_string(), PositionAccumulator::default(), r1),
            ("lane2.cbq".to_string(), PositionAccumulator::default(), r2),
        ])
        .unwrap();

        assert_eq!(result.geometry_string, "[barcode][umi:12] | [gex]");
        assert_eq!(result.remap_window, 5); // max of 2 and 5
        assert_eq!(result.total_reads_sampled, 18000); // 10000 + 8000
        assert_eq!(result.per_file_results.len(), 2);
        assert_eq!(result.per_file_results[0].remap_window, 2);
        assert_eq!(result.per_file_results[1].remap_window, 5);
    }

    #[test]
    fn test_validate_and_aggregate_mismatched_geometries() {
        let r1 = build_detection_result("[barcode][umi:12] | [gex]", 2, 10000);
        let r2 = build_detection_result("[barcode][umi:12] | [:5][gex]", 3, 10000);
        let err = validate_and_aggregate(vec![
            ("lane1.cbq".to_string(), PositionAccumulator::default(), r1),
            ("lane2.cbq".to_string(), PositionAccumulator::default(), r2),
        ])
        .unwrap_err();

        let msg = err.to_string();
        assert!(
            msg.contains("Geometry mismatch"),
            "error should mention mismatch: {msg}"
        );
        assert!(
            msg.contains("lane1.cbq"),
            "error should name first file: {msg}"
        );
        assert!(
            msg.contains("lane2.cbq"),
            "error should name second file: {msg}"
        );
        assert!(
            msg.contains("[barcode][umi:12] | [gex]"),
            "error should show first geometry: {msg}"
        );
        assert!(
            msg.contains("[barcode][umi:12] | [:5][gex]"),
            "error should show second geometry: {msg}"
        );
    }

    #[test]
    fn test_validate_and_aggregate_three_files_one_mismatch() {
        let r1 = build_detection_result("[barcode][umi:12] | [gex]", 2, 10000);
        let r2 = build_detection_result("[barcode][umi:12] | [gex]", 3, 10000);
        let r3 = build_detection_result("[barcode][umi:12] | [:5][gex]", 4, 10000);
        let err = validate_and_aggregate(vec![
            ("a.cbq".to_string(), PositionAccumulator::default(), r1),
            ("b.cbq".to_string(), PositionAccumulator::default(), r2),
            ("c.cbq".to_string(), PositionAccumulator::default(), r3),
        ])
        .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("Geometry mismatch"), "{msg}");
        assert!(
            msg.contains("c.cbq"),
            "error should list the mismatched file: {msg}"
        );
    }

    // -------------------------------------------------------------------
    // find_best_positions top-5 cap
    // -------------------------------------------------------------------

    #[test]
    fn test_find_best_positions_top_five_cap() {
        // 7 entries for the same (Component, ReadMate) at distinct positions
        // with monotonically-decreasing counts. `find_best_positions` should
        // cap `top_positions` at 5.
        let acc = build_accumulator(
            &[
                (Component::Gex, ReadMate::R2, 0, 7000),
                (Component::Gex, ReadMate::R2, 1, 6000),
                (Component::Gex, ReadMate::R2, 2, 5000),
                (Component::Gex, ReadMate::R2, 3, 4000),
                (Component::Gex, ReadMate::R2, 4, 3000),
                (Component::Gex, ReadMate::R2, 5, 2000),
                (Component::Gex, ReadMate::R2, 6, 1000),
            ],
            10000,
        );
        let assignments = find_best_positions(&acc, &[Component::Gex]);
        assert_eq!(assignments.len(), 1);
        let top = &assignments[0].top_positions;
        assert_eq!(
            top.len(),
            5,
            "top_positions must be capped at 5, got {}",
            top.len()
        );
        // Sorted by count descending: 7000, 6000, 5000, 4000, 3000.
        let counts: Vec<usize> = top.iter().map(|&(_, _, c)| c).collect();
        assert_eq!(counts, vec![7000, 6000, 5000, 4000, 3000]);
    }

    // -------------------------------------------------------------------
    // resolve_overlaps no-alternative bail
    // -------------------------------------------------------------------

    #[test]
    fn test_resolve_overlaps_no_alternative_bail() {
        // Winner: Gex at R2:0 with seq_len=100, count=10000 -- covers R2 0..100.
        // Loser:  Probe at R2:5 with seq_len=8, count=3000.
        //         Loser's top_positions at R2:5, R2:30, R2:70 all fall within
        //         R2:0..100, so every alternative overlaps with the winner.
        //         alt_len = 8 (loser's seq_len); 5+8=13, 30+8=38, 70+8=78 -- all < 100.
        //         resolve_overlaps cannot find a non-overlapping alternative -> bails.
        let mut assignments = vec![
            ComponentAssignment {
                component: Component::Gex,
                mate: ReadMate::R2,
                position: 0,
                seq_len: Some(100),
                count: 10000,
                top_positions: vec![(ReadMate::R2, 0, 10000)],
            },
            ComponentAssignment {
                component: Component::Probe,
                mate: ReadMate::R2,
                position: 5,
                seq_len: Some(8),
                count: 3000,
                top_positions: vec![
                    (ReadMate::R2, 5, 3000),
                    (ReadMate::R2, 30, 2000),
                    (ReadMate::R2, 70, 1000),
                ],
            },
        ];

        let err = resolve_overlaps(&mut assignments).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("cannot find non-overlapping position"),
            "expected no-alternative bail, got: {msg}"
        );
    }

    // -------------------------------------------------------------------
    // Windowed match count/proportion tests
    // -------------------------------------------------------------------

    /// Test 1: tight distribution -- all gex hits at the same position.
    /// Windowed count must equal single-position `match_count`.
    #[test]
    fn test_windowed_proportion_tight_distribution() {
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 0, 5000),
            ],
            10_000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 10_000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap();

        let gex = result
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        assert_eq!(gex.match_count, 5000);
        assert_eq!(
            gex.windowed_match_count, gex.match_count,
            "tight distribution: windowed_match_count must equal match_count",
        );
        assert!(
            (gex.windowed_match_proportion - gex.match_proportion).abs() < 1e-12,
            "tight distribution: windowed_match_proportion must equal match_proportion",
        );
    }

    /// Test 2: spread within window -- gex at three positions clustered tightly
    /// enough for W=1; assert windowed count sums all three positions.
    #[test]
    fn test_windowed_proportion_spread_within_window() {
        // 20000 reads. min_hits = ceil(20000 * 0.01) = 200.
        // Best gex position: R2:66 with 5000.
        // Walk above 66: 67=3000>=200, 68=0<200 stop. farthest_above = 67.
        // Walk below 66: 65=2000>=200, 64=0<200 stop. farthest_below = 65.
        // W = max(67-66, 66-65) = 1.
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 65, 2000),
                (Component::Gex, ReadMate::R2, 66, 5000),
                (Component::Gex, ReadMate::R2, 67, 3000),
            ],
            20_000,
        );
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));

        let config = DetectionConfig {
            num_reads: 20_000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap();
        assert_eq!(result.remap_window, 1, "expected W=1 for this distribution");

        let gex = result
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        assert_eq!(gex.match_count, 5000, "single-position match_count is 5000");
        assert!(
            (gex.match_proportion - 0.25).abs() < 1e-12,
            "single-position proportion is 5000/20000 = 0.25",
        );
        // Windowed sum at W=1 centered on 66: [65,67] = 2000 + 5000 + 3000 = 10000.
        assert_eq!(
            gex.windowed_match_count, 10_000,
            "windowed sum should aggregate all three positions in [65,67]",
        );
        assert!(
            (gex.windowed_match_proportion - 0.5).abs() < 1e-12,
            "windowed proportion is 10000/20000 = 0.5",
        );
    }

    /// Test 3: clipped at window edge -- confirms boundary inclusion at pos+W
    /// and exclusion at pos+W+1. Two scenarios:
    ///   variant A: fillers carry W out to 4, so an alt at pos+4 IS included;
    ///   variant B: no fillers, W clamps to 1, so the same alt is EXCLUDED.
    #[test]
    fn test_windowed_proportion_clipped_at_window_edge() {
        let mut seq_lens = HashMap::new();
        seq_lens.insert(Component::Barcode, Some(16));
        seq_lens.insert(Component::Gex, Some(50));
        let config = DetectionConfig {
            num_reads: 20_000,
            min_proportion: 0.10,
            remap_min_proportion: DEFAULT_REMAP_MIN_PROPORTION,
            num_threads: 1,
        };

        // Variant A: fillers at 67-69 carry W to 4 (range 67..=70 reaches min_hits=200).
        // Walk above 66: 67=200, 68=200, 69=200, 70=4000, 71=0<200 stop. farthest_above=70.
        // Walk below 66: 65=0<200 stop. window = max(4, 0) = 4.
        let acc_a = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 66, 5000),
                (Component::Gex, ReadMate::R2, 67, 200),
                (Component::Gex, ReadMate::R2, 68, 200),
                (Component::Gex, ReadMate::R2, 69, 200),
                (Component::Gex, ReadMate::R2, 70, 4000),
            ],
            20_000,
        );
        let result_a =
            infer_geometry(&acc_a, DetectionMode::Gex, false, &seq_lens, &config).unwrap();
        assert_eq!(result_a.remap_window, 4, "variant A: expected W=4");
        let gex_a = result_a
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        // Windowed [62,70] = 0+0+0+0+5000+200+200+200+4000 = 9600.
        // Confirms inclusive upper boundary: R2:70 at pos+W=70 IS counted.
        assert_eq!(
            gex_a.windowed_match_count, 9600,
            "variant A: windowed sum at W=4 must include R2:70 (upper boundary)",
        );

        // Variant B: no fillers, W clamps to 1. R2:70 outside [65,67].
        let acc_b = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 66, 5000),
                (Component::Gex, ReadMate::R2, 70, 4000),
            ],
            20_000,
        );
        let result_b =
            infer_geometry(&acc_b, DetectionMode::Gex, false, &seq_lens, &config).unwrap();
        assert_eq!(result_b.remap_window, 1, "variant B: expected W=1");
        let gex_b = result_b
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        // Windowed [65,67] = 0 + 5000 + 0 = 5000. R2:70 is OUT of [65,67].
        assert_eq!(
            gex_b.windowed_match_count, 5000,
            "variant B: alt at pos+4 must be excluded when W=1",
        );
    }

    /// Test 4: same-mate-only -- noise hits on the opposite mate at the same
    /// position must be excluded from the windowed sum. Exercises the
    /// `*m == mate` filter in `PositionAccumulator::windowed_count`.
    #[test]
    fn test_windowed_proportion_same_mate_only() {
        let acc = build_accumulator(
            &[
                (Component::Gex, ReadMate::R1, 66, 1000), // wrong mate noise
                (Component::Gex, ReadMate::R2, 66, 5000), // best on right mate
            ],
            10_000,
        );

        // Asking for R2 must NOT pick up the R1:66 contribution.
        assert_eq!(
            acc.windowed_count(Component::Gex, ReadMate::R2, 66, 1),
            5000,
            "R1:66 noise must not leak into the R2 windowed sum",
        );
        // Mirror check: asking for R1 must NOT pick up the R2:66 contribution.
        assert_eq!(
            acc.windowed_count(Component::Gex, ReadMate::R1, 66, 1),
            1000,
            "R2:66 must not leak into the R1 windowed sum",
        );
    }

    /// Test 5: saturating lower bound -- when `best_pos < window`, the lower
    /// bound clips to 0 instead of underflowing. Exercises `saturating_sub`.
    #[test]
    fn test_windowed_proportion_saturating_lo() {
        let acc = build_accumulator(
            &[
                (Component::Gex, ReadMate::R2, 0, 1000),
                (Component::Gex, ReadMate::R2, 1, 1000),
                (Component::Gex, ReadMate::R2, 2, 5000),
            ],
            10_000,
        );
        // best_pos=2, window=5 -> lo = 2.saturating_sub(5) = 0, hi = 7.
        // Walk [0, 7]: 1000 + 1000 + 5000 = 7000.
        assert_eq!(
            acc.windowed_count(Component::Gex, ReadMate::R2, 2, 5),
            7000,
            "saturating_sub must clip the lower bound to 0",
        );
    }

    /// Test 6: cross-file aggregation -- merged-accumulator re-walk produces
    /// a different (and correct) windowed count than sum-of-per-file would.
    /// File A has a single straggler at R2:68 that is OUTSIDE file A's
    /// per-file window (`W_A=1`) but INSIDE the merged window (`W=2`). The
    /// merged-re-walk picks it up; sum-of-per-file would not.
    #[test]
    #[allow(clippy::too_many_lines, clippy::similar_names)]
    fn test_validate_and_aggregate_windowed_cross_file() {
        // File A: 5000@66, 500@68. Per-file W=1 (68 is 2 away, neighbors 67,69 empty).
        let acc_a = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 66, 5000),
                (Component::Gex, ReadMate::R2, 68, 500),
            ],
            10_000,
        );
        let result_a = DetectionResult {
            geometry: Geometry {
                r1: Read { regions: vec![] },
                r2: Read { regions: vec![] },
            },
            geometry_string: "[barcode][umi:12] | [gex]".to_string(),
            remap_window: 1,
            evidence: vec![
                ComponentEvidence {
                    component: Component::Barcode,
                    mate: ReadMate::R1,
                    position: 0,
                    seq_len: Some(16),
                    match_count: 5000,
                    match_proportion: 0.5,
                    windowed_match_count: 5000,
                    windowed_match_proportion: 0.5,
                    top_positions: vec![(ReadMate::R1, 0, 5000)],
                },
                ComponentEvidence {
                    component: Component::Gex,
                    mate: ReadMate::R2,
                    position: 66,
                    seq_len: Some(50),
                    match_count: 5000,
                    match_proportion: 0.5,
                    // Per-file W=1: [65,67] on file A's acc = 0 + 5000 + 0 = 5000.
                    windowed_match_count: 5000,
                    windowed_match_proportion: 0.5,
                    top_positions: vec![(ReadMate::R2, 66, 5000), (ReadMate::R2, 68, 500)],
                },
            ],
            total_reads_sampled: 10_000,
            per_file_results: Vec::new(),
        };

        // File B: 3000@66, 2000@67, 2000@68. Per-file W=2.
        let acc_b = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Gex, ReadMate::R2, 66, 3000),
                (Component::Gex, ReadMate::R2, 67, 2000),
                (Component::Gex, ReadMate::R2, 68, 2000),
            ],
            10_000,
        );
        let result_b = DetectionResult {
            geometry: Geometry {
                r1: Read { regions: vec![] },
                r2: Read { regions: vec![] },
            },
            geometry_string: "[barcode][umi:12] | [gex]".to_string(),
            remap_window: 2,
            evidence: vec![
                ComponentEvidence {
                    component: Component::Barcode,
                    mate: ReadMate::R1,
                    position: 0,
                    seq_len: Some(16),
                    match_count: 5000,
                    match_proportion: 0.5,
                    windowed_match_count: 5000,
                    windowed_match_proportion: 0.5,
                    top_positions: vec![(ReadMate::R1, 0, 5000)],
                },
                ComponentEvidence {
                    component: Component::Gex,
                    mate: ReadMate::R2,
                    position: 66,
                    seq_len: Some(50),
                    match_count: 3000,
                    match_proportion: 0.3,
                    // Per-file W=2: [64,68] on file B's acc = 0+0+3000+2000+2000 = 7000.
                    windowed_match_count: 7000,
                    windowed_match_proportion: 0.7,
                    top_positions: vec![
                        (ReadMate::R2, 66, 3000),
                        (ReadMate::R2, 67, 2000),
                        (ReadMate::R2, 68, 2000),
                    ],
                },
            ],
            total_reads_sampled: 10_000,
            per_file_results: Vec::new(),
        };

        let aggregated = validate_and_aggregate(vec![
            ("a.cbq".to_string(), acc_a, result_a),
            ("b.cbq".to_string(), acc_b, result_b),
        ])
        .unwrap();

        assert_eq!(
            aggregated.remap_window, 2,
            "max_remap_window = max(1,2) = 2"
        );
        assert_eq!(aggregated.total_reads_sampled, 20_000);

        // Merged accumulator at R2: 66->8000, 67->2000, 68->2500.
        // Re-walk at W=2 centered on R2:66: [64,68] = 0+0+8000+2000+2500 = 12500.
        // Sum-of-per-file (the wrong approach) would be 5000+7000 = 12000.
        let gex = aggregated
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        assert_eq!(gex.match_count, 8000, "aggregated match_count is 5000+3000");
        assert_eq!(
            gex.windowed_match_count, 12_500,
            "merged re-walk must produce 12500 (catches the straggler at R2:68 in file A)",
        );
        assert_ne!(
            gex.windowed_match_count, 12_000,
            "merged re-walk must NOT equal sum-of-per-file (12000)",
        );
        assert!(
            (gex.windowed_match_proportion - 0.625).abs() < 1e-12,
            "windowed proportion = 12500/20000 = 0.625, got {}",
            gex.windowed_match_proportion,
        );

        // Per-file windowed values are cloned, not recomputed -- they must
        // preserve each file's own W.
        let per_file_a_gex = aggregated.per_file_results[0]
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        assert_eq!(per_file_a_gex.windowed_match_count, 5000);
        let per_file_b_gex = aggregated.per_file_results[1]
            .evidence
            .iter()
            .find(|e| e.component == Component::Gex)
            .unwrap();
        assert_eq!(per_file_b_gex.windowed_match_count, 7000);
    }

    /// Test 7: upper boundary inclusion -- directly exercises the `*p <= hi`
    /// filter. With `best_pos=10`, `window=3`, `hi=13`: `pos=13` IS counted,
    /// `pos=14` is NOT. Distinct from Test 5 (which exercises the saturating
    /// lower bound) and from Test 3 (which exercises upper-bound inclusion
    /// through `infer_geometry`).
    #[test]
    fn test_windowed_count_helper_includes_upper_boundary() {
        let acc = build_accumulator(
            &[
                (Component::Gex, ReadMate::R2, 10, 100), // best
                (Component::Gex, ReadMate::R2, 13, 50),  // = hi, inclusive
                (Component::Gex, ReadMate::R2, 14, 99),  // = hi+1, excluded
            ],
            10_000,
        );
        // best_pos=10, window=3 -> lo=7, hi=13. Walk [7,13]: 100 + 50 = 150.
        assert_eq!(
            acc.windowed_count(Component::Gex, ReadMate::R2, 10, 3),
            150,
            "upper boundary at hi must be inclusive; hi+1 must be excluded",
        );
    }
}
