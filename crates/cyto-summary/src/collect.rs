//! Collect a `SampleReport` from a finished cyto output directory.
//!
//! Every read is best-effort: a missing or malformed stat file degrades the
//! corresponding panel rather than failing the whole report. Problems are
//! recorded in `SampleReport::notes` and surfaced in the rendered output.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cyto_io::match_input_transparent;
use log::{debug, warn};
use serde::de::DeserializeOwned;

use crate::model::{
    AssignmentRaw, AssignmentSummary, KneePoint, LibraryRaw, MappingRaw, ProbeSummary, RunRaw,
    SampleKind, SampleReport, TimingRow, UmiRaw,
};

/// Return true if `path` looks like a finished cyto sample directory.
pub fn is_sample_dir(path: &Path) -> bool {
    path.is_dir()
        && (path.join("stats/mapping_map.json").is_file()
            || path.join("stats/mapping_run.json").is_file()
            || path.join(".done").is_file())
}

/// Expand each input into concrete sample directories.
///
/// An input that is itself a sample directory is used directly; otherwise its
/// immediate children are scanned. Order is preserved and duplicates removed.
pub fn discover_samples(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        let canon = p.canonicalize().unwrap_or(p);
        if !out.contains(&canon) {
            out.push(canon);
        }
    };

    for input in inputs {
        if is_sample_dir(input) {
            push(input.clone(), &mut out);
            continue;
        }
        if !input.is_dir() {
            warn!("Skipping {} (not a directory)", input.display());
            continue;
        }
        let mut children: Vec<PathBuf> = std::fs::read_dir(input)
            .with_context(|| format!("Unable to read directory {}", input.display()))?
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| is_sample_dir(p))
            .collect();
        children.sort();
        if children.is_empty() {
            warn!("No cyto sample directories found under {}", input.display());
        }
        for child in children {
            push(child, &mut out);
        }
    }
    Ok(out)
}

/// Read and deserialize a JSON stat file, returning `None` when it is absent.
///
/// A malformed file is not fatal: it logs, records a note, and yields `None`.
fn read_json<T: DeserializeOwned>(path: &Path, notes: &mut Vec<String>) -> Option<T> {
    if !path.is_file() {
        return None;
    }
    match std::fs::File::open(path)
        .map_err(anyhow::Error::from)
        .and_then(|f| {
            serde_json::from_reader::<_, T>(std::io::BufReader::new(f)).map_err(Into::into)
        }) {
        Ok(v) => Some(v),
        Err(e) => {
            warn!("Failed to parse {}: {e}", path.display());
            notes.push(format!("Failed to parse {}: {e}", path.display()));
            None
        }
    }
}

/// Detect whether a sample is GEX or CRISPR.
fn detect_kind(path: &Path) -> SampleKind {
    if let Ok(done) = std::fs::read_to_string(path.join(".done")) {
        let head = done.trim_start();
        if head.starts_with("GexMapping") {
            return SampleKind::Gex;
        }
        if head.starts_with("CrisprMapping") {
            return SampleKind::Crispr;
        }
    }
    if path.join("stats/assignments").is_dir() {
        SampleKind::Crispr
    } else if path.join("stats/filtering").is_dir() {
        SampleKind::Gex
    } else {
        SampleKind::Unknown
    }
}

/// Aggregates derived from a single `reads.tsv.zst` file.
struct ReadsAgg {
    n_barcodes: u64,
    total_reads: u64,
    total_umis: u64,
    median_reads: f64,
    median_umis: f64,
    knee: Vec<KneePoint>,
}

fn median_sorted(sorted: &[u64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
    }
}

/// Sample `n_points` log-spaced ranks from a UMI-count vector sorted descending.
#[allow(clippy::cast_sign_loss)] // `rank` is clamped to >= 1.0 before the cast
fn knee_points(umis_desc: &[u64], n_points: usize) -> Vec<KneePoint> {
    let n = umis_desc.len();
    if n == 0 || n_points == 0 {
        return Vec::new();
    }
    if n <= n_points {
        return umis_desc
            .iter()
            .enumerate()
            .map(|(i, &u)| KneePoint {
                rank: i as u64 + 1,
                umis: u,
            })
            .collect();
    }
    let ln_n = (n as f64).ln();
    let mut out = Vec::with_capacity(n_points);
    let mut last: u64 = 0;
    for i in 0..n_points {
        let frac = i as f64 / (n_points - 1) as f64;
        let rank = (ln_n * frac).exp().round().max(1.0) as u64;
        let rank = rank.min(n as u64);
        if rank == last {
            continue;
        }
        last = rank;
        out.push(KneePoint {
            rank,
            umis: umis_desc[rank as usize - 1],
        });
    }
    out
}

/// Parse a `reads.tsv.zst` (columns: barcode, `n_umis`, `n_reads`).
fn read_reads(path: &Path, rank_points: usize) -> Result<ReadsAgg> {
    let handle = match_input_transparent(Some(path))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_reader(handle);

    let mut umis: Vec<u64> = Vec::new();
    let mut reads: Vec<u64> = Vec::new();
    let mut total_reads: u64 = 0;
    let mut total_umis: u64 = 0;

    let mut record = csv::StringRecord::new();
    while rdr.read_record(&mut record)? {
        // columns: 0 = barcode, 1 = n_umis, 2 = n_reads
        let n_umis: u64 = record.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let n_reads: u64 = record.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        total_umis += n_umis;
        total_reads += n_reads;
        umis.push(n_umis);
        reads.push(n_reads);
    }

    reads.sort_unstable();
    let median_reads = median_sorted(&reads);
    let median_umis = {
        let mut u = umis.clone();
        u.sort_unstable();
        median_sorted(&u)
    };

    umis.sort_unstable_by(|a, b| b.cmp(a));
    let knee = knee_points(&umis, rank_points);

    Ok(ReadsAgg {
        n_barcodes: umis.len() as u64,
        total_reads,
        total_umis,
        median_reads,
        median_umis,
        knee,
    })
}

/// Enumerate probe basenames from `stats/reads/*.reads.tsv.zst`.
fn probe_names(path: &Path) -> Vec<String> {
    let reads_dir = path.join("stats/reads");
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&reads_dir) {
        for entry in entries.filter_map(std::result::Result::ok) {
            let fname = entry.file_name().to_string_lossy().to_string();
            if let Some(base) = fname.strip_suffix(".reads.tsv.zst") {
                names.push(base.to_string());
            }
        }
    }
    names.sort();
    names
}

fn saturation(total_reads: u64, total_umis: u64) -> Option<f64> {
    if total_reads == 0 {
        None
    } else {
        Some(1.0 - (total_umis as f64 / total_reads as f64))
    }
}

fn assignment_summary(raw: &AssignmentRaw) -> AssignmentSummary {
    let mut moi_distribution: Vec<(i64, u64)> = raw
        .mois
        .iter()
        .copied()
        .zip(raw.moi_counts.iter().copied())
        .collect();
    moi_distribution.sort_by_key(|(moi, _)| *moi);
    let assigned_frac = if raw.n_tested == 0 {
        0.0
    } else {
        raw.n_assigned as f64 / raw.n_tested as f64
    };
    AssignmentSummary {
        n_tested: raw.n_tested,
        n_assigned: raw.n_assigned,
        n_unassigned: raw.n_unassigned,
        assigned_frac,
        dominant_moi: raw.dominant_moi,
        moi_distribution,
    }
}

/// Read one `.timings.tsv` into rows (empty on absence/failure).
fn read_timings(path: &Path) -> Vec<TimingRow> {
    let timings_path = path.join(".timings.tsv");
    if !timings_path.is_file() {
        return Vec::new();
    }
    match csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_path(&timings_path)
    {
        Ok(mut rdr) => rdr
            .deserialize::<TimingRow>()
            .filter_map(std::result::Result::ok)
            .collect(),
        Err(e) => {
            warn!("Failed to read {}: {e}", timings_path.display());
            Vec::new()
        }
    }
}

/// Collect all available information from a single sample directory.
pub fn collect_sample(path: &Path, rank_points: usize) -> SampleReport {
    debug!("Collecting sample {}", path.display());
    let mut notes: Vec<String> = Vec::new();

    let sample = path.file_name().map_or_else(
        || path.display().to_string(),
        |s| s.to_string_lossy().to_string(),
    );
    let kind = detect_kind(path);

    let mapping: Option<MappingRaw> = read_json(&path.join("stats/mapping_map.json"), &mut notes);
    let libraries: Vec<LibraryRaw> =
        read_json(&path.join("stats/mapping_lib.json"), &mut notes).unwrap_or_default();
    let runtime_sec = read_json::<Vec<RunRaw>>(&path.join("stats/mapping_run.json"), &mut notes)
        .map(|runs| runs.iter().map(|r| r.elapsed_sec).sum());

    let mut probes: Vec<ProbeSummary> = Vec::new();
    for name in probe_names(path) {
        let reads_path = path.join(format!("stats/reads/{name}.reads.tsv.zst"));
        let agg = match read_reads(&reads_path, rank_points) {
            Ok(agg) => agg,
            Err(e) => {
                warn!("Failed to read {}: {e}", reads_path.display());
                notes.push(format!("Failed to read {}: {e}", reads_path.display()));
                continue;
            }
        };

        let umi_corrected_frac =
            read_json::<UmiRaw>(&path.join(format!("stats/umi/{name}.umi.json")), &mut notes)
                .map(|u| u.fraction_corrected);

        let assignment = read_json::<AssignmentRaw>(
            &path.join(format!("stats/assignments/{name}.json")),
            &mut notes,
        )
        .map(|raw| assignment_summary(&raw));

        probes.push(ProbeSummary {
            name,
            n_barcodes: agg.n_barcodes,
            total_reads: agg.total_reads,
            total_umis: agg.total_umis,
            saturation: saturation(agg.total_reads, agg.total_umis).unwrap_or(0.0),
            median_reads_per_barcode: agg.median_reads,
            median_umis_per_barcode: agg.median_umis,
            umi_corrected_frac,
            assignment,
            knee: agg.knee,
        });
    }

    let total_reads: u64 = probes.iter().map(|p| p.total_reads).sum();
    let total_umis: u64 = probes.iter().map(|p| p.total_umis).sum();
    let overall_saturation = saturation(total_reads, total_umis);
    let timings = read_timings(path);

    if mapping.is_none() && probes.is_empty() {
        notes.push("No mapping stats or per-probe reads found in this directory".to_string());
    }

    SampleReport {
        sample,
        path: path.display().to_string(),
        kind,
        mapping,
        libraries,
        runtime_sec,
        probes,
        timings,
        total_reads,
        total_umis,
        overall_saturation,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::{knee_points, median_sorted, saturation};

    #[test]
    fn median_odd_even_empty() {
        assert!((median_sorted(&[]) - 0.0).abs() < 1e-9);
        assert!((median_sorted(&[5]) - 5.0).abs() < 1e-9);
        assert!((median_sorted(&[1, 3]) - 2.0).abs() < 1e-9);
        assert!((median_sorted(&[1, 2, 3]) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn saturation_matches_definition() {
        assert!(saturation(0, 0).is_none());
        // 100 reads, 25 umis => 75% saturation
        let s = saturation(100, 25).unwrap();
        assert!((s - 0.75).abs() < 1e-9);
    }

    #[test]
    fn knee_points_are_monotone_and_bounded() {
        let umis: Vec<u64> = (0..10_000).rev().map(|x| x as u64).collect();
        let pts = knee_points(&umis, 50);
        assert!(!pts.is_empty());
        assert!(pts.len() <= 50);
        // ranks strictly increasing, within bounds
        for w in pts.windows(2) {
            assert!(w[1].rank > w[0].rank);
        }
        assert_eq!(pts.first().unwrap().rank, 1);
        assert!(pts.last().unwrap().rank <= umis.len() as u64);
    }

    #[test]
    fn knee_points_small_input_returns_all() {
        let umis = vec![9, 7, 5];
        let pts = knee_points(&umis, 100);
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0].rank, 1);
        assert_eq!(pts[0].umis, 9);
    }
}
