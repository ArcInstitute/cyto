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

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Configuration for geometry auto-detection.
pub struct DetectionConfig {
    pub num_reads: usize,
    pub min_proportion: f64,
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
    /// Top positions by match count (for logging alternative candidates).
    pub top_positions: Vec<(ReadMate, usize, usize)>,
}

/// Full detection result.
#[derive(Debug)]
pub struct DetectionResult {
    pub geometry: Geometry,
    pub geometry_string: String,
    pub remap_window: usize,
    pub evidence: Vec<ComponentEvidence>,
    pub total_reads_sampled: usize,
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
    limit: usize,
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
        if self.shared.counter.fetch_add(1, Ordering::Relaxed) >= self.shared.limit {
            return;
        }
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

/// Sample reads for GEX detection.
///
/// Moves the mappers into shared state. Returns the accumulated position data.
fn sample_gex_reads(
    whitelist: WhitelistMapper<Unpositioned>,
    gex: GexMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    num_reads: usize,
) -> Result<PositionAccumulator> {
    let shared = Arc::new(GexSharedState {
        whitelist,
        gex,
        probe,
        global_accumulator: Mutex::new(PositionAccumulator::default()),
        counter: AtomicUsize::new(0),
        limit: num_reads,
    });

    let mut proc = GexDetectionProcessor {
        shared: Arc::clone(&shared),
        local: PositionAccumulator::default(),
        tid: 0,
    };

    if input.is_binseq() {
        let reader = BinseqReader::new(&input.inputs[0])?;
        let n = reader.num_records()?.min(num_reads);
        if n > 0 {
            reader.process_parallel_range(proc, 1, 0..n)?;
        }
    } else {
        let collection = input.to_paraseq_collection()?;
        collection.process_parallel_paired(&mut proc, 1, None)?;
        // Flush remaining local data (paraseq passes &mut, so we can flush here)
        proc.flush();
    }

    // Extract accumulated data
    let accumulator = shared.global_accumulator.lock().clone();
    Ok(accumulator)
}

/// Sample reads for CRISPR detection.
fn sample_crispr_reads(
    whitelist: WhitelistMapper<Unpositioned>,
    crispr: CrisprMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    num_reads: usize,
) -> Result<PositionAccumulator> {
    let shared = Arc::new(CrisprSharedState {
        whitelist,
        crispr,
        probe,
        global_accumulator: Mutex::new(PositionAccumulator::default()),
        counter: AtomicUsize::new(0),
        limit: num_reads,
    });

    let mut proc = CrisprDetectionProcessor {
        shared: Arc::clone(&shared),
        local: PositionAccumulator::default(),
        tid: 0,
    };

    if input.is_binseq() {
        let reader = BinseqReader::new(&input.inputs[0])?;
        let n = reader.num_records()?.min(num_reads);
        if n > 0 {
            reader.process_parallel_range(proc, 1, 0..n)?;
        }
    } else {
        let collection = input.to_paraseq_collection()?;
        collection.process_parallel_paired(&mut proc, 1, None)?;
        proc.flush();
    }

    let accumulator = shared.global_accumulator.lock().clone();
    Ok(accumulator)
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

    if total_reads < 1000 {
        warn!("Only {total_reads} reads sampled for geometry detection; confidence may be low.");
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
                 below threshold {:.2}. Auto-detection failed.\n\
                 Provide --geometry or --preset manually.",
                assignment.component,
                proportion,
                assignment.count,
                total_reads,
                config.min_proportion,
            );
        }
    }

    // Build evidence
    let evidence: Vec<ComponentEvidence> = assignments
        .iter()
        .map(|a| ComponentEvidence {
            component: a.component,
            mate: a.mate,
            position: a.position,
            seq_len: a.seq_len,
            match_count: a.count,
            match_proportion: a.count as f64 / total_reads as f64,
            top_positions: a.top_positions.clone(),
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
    let umi_len: usize = 12;

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

    let remap_window = estimate_remap_window(accumulator, &components);

    Ok(DetectionResult {
        geometry,
        geometry_string,
        remap_window,
        evidence,
        total_reads_sampled: total_reads,
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

/// Estimate the optimal remap window from position distributions.
///
/// For each component, finds the best mate and computes the spread of positions
/// on that mate only (positions on the other mate are spurious and must be excluded).
fn estimate_remap_window(accumulator: &PositionAccumulator, components: &[Component]) -> usize {
    let mut max_range = 0usize;

    for &comp in components {
        // Find the best (mate, position) for this component.
        let best_entry = accumulator
            .counts
            .iter()
            .filter(|((c, _, _), _)| *c == comp)
            .max_by_key(|&(_, count)| count);

        let Some((&(_, best_mate, _), &best_count)) = best_entry else {
            continue;
        };

        if best_count == 0 {
            continue;
        }

        #[allow(clippy::cast_sign_loss)] // product is always non-negative
        let threshold = (best_count as f64 * 0.05) as usize;

        // Only consider positions on the same mate as the best hit.
        let significant_positions: Vec<usize> = accumulator
            .counts
            .iter()
            .filter(|((c, mate, _), count)| {
                *c == comp && *mate == best_mate && **count >= threshold
            })
            .map(|((_, _, pos), _)| *pos)
            .collect();

        if significant_positions.len() > 1 {
            let min_pos = *significant_positions.iter().min().unwrap();
            let max_pos = *significant_positions.iter().max().unwrap();
            let range = max_pos - min_pos;
            max_range = max_range.max(range);
        }
    }

    if max_range == 0 {
        1
    } else {
        1.max(max_range.div_ceil(2))
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect GEX geometry by sampling reads and scanning for component positions.
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
    info!(
        "Auto-detecting GEX geometry from {} reads...",
        config.num_reads
    );

    let mut component_seq_lens: HashMap<Component, Option<usize>> = HashMap::new();
    component_seq_lens.insert(Component::Barcode, Some(whitelist.seq_len()));
    component_seq_lens.insert(Component::Gex, Some(gex.seq_len()));
    if let Some(ref p) = probe {
        component_seq_lens.insert(Component::Probe, Some(p.seq_len()));
    }

    let has_probe = probe.is_some();
    let accumulator = sample_gex_reads(whitelist, gex, probe, input, config.num_reads)?;

    infer_geometry(
        &accumulator,
        DetectionMode::Gex,
        has_probe,
        &component_seq_lens,
        config,
    )
}

/// Detect CRISPR geometry by sampling reads and scanning for component positions.
///
/// The mappers are moved into the detection processor and consumed.
pub fn detect_crispr_geometry(
    whitelist: WhitelistMapper<Unpositioned>,
    crispr: CrisprMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
    config: &DetectionConfig,
) -> Result<DetectionResult> {
    info!(
        "Auto-detecting CRISPR geometry from {} reads...",
        config.num_reads
    );

    let mut component_seq_lens: HashMap<Component, Option<usize>> = HashMap::new();
    component_seq_lens.insert(Component::Barcode, Some(whitelist.seq_len()));
    component_seq_lens.insert(Component::Anchor, crispr.anchor_len());
    component_seq_lens.insert(Component::Protospacer, Some(crispr.protospacer_len()));
    if let Some(ref p) = probe {
        component_seq_lens.insert(Component::Probe, Some(p.seq_len()));
    }

    let has_probe = probe.is_some();
    let accumulator = sample_crispr_reads(whitelist, crispr, probe, input, config.num_reads)?;

    infer_geometry(
        &accumulator,
        DetectionMode::Crispr,
        has_probe,
        &component_seq_lens,
        config,
    )
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
        };

        let result = infer_geometry(&acc, DetectionMode::Gex, false, &seq_lens, &config).unwrap();

        // Should have skip:3, barcode, umi:12 on R1
        assert_eq!(
            result.geometry_string,
            "[:3][barcode][umi:12] | [gex]"
        );
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
        // All matches at the same position -> window 1
        let acc = build_accumulator(
            &[(Component::Barcode, ReadMate::R1, 0, 5000)],
            10000,
        );
        let window = estimate_remap_window(&acc, &[Component::Barcode]);
        assert_eq!(window, 1);
    }

    #[test]
    fn test_remap_window_spread_distribution() {
        // Matches spread across positions 0..4 -> range=4, window=ceil(4/2)=2
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Barcode, ReadMate::R1, 1, 4000),
                (Component::Barcode, ReadMate::R1, 2, 3000),
                (Component::Barcode, ReadMate::R1, 4, 1000),
            ],
            10000,
        );
        let window = estimate_remap_window(&acc, &[Component::Barcode]);
        assert_eq!(window, 2);
    }

    #[test]
    fn test_remap_window_insignificant_outliers_ignored() {
        // Main at pos 0 (5000), outlier at pos 100 (10, below 5% of 5000=250)
        let acc = build_accumulator(
            &[
                (Component::Barcode, ReadMate::R1, 0, 5000),
                (Component::Barcode, ReadMate::R1, 100, 10),
            ],
            10000,
        );
        let window = estimate_remap_window(&acc, &[Component::Barcode]);
        assert_eq!(window, 1);
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
        assert_eq!(format_geometry_string(&geometry), "[barcode][umi:12] | [gex]");
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
        assert_eq!(
            a.counts[&(Component::Barcode, ReadMate::R1, 0)],
            3
        );
        assert_eq!(
            a.counts[&(Component::Barcode, ReadMate::R1, 5)],
            1
        );
    }
}
