//! Data model for cyto QC reports.
//!
//! Two families of structs live here:
//! - `*Raw` types mirror the on-disk JSON stat files (deserialization targets).
//! - The report types (`SampleReport`, `ProbeSummary`, ...) are the aggregated,
//!   serialized-to-sidecar view rendered into HTML.

use serde::{Deserialize, Serialize};

/// Mirror of `stats/mapping_map.json` (`cyto-map` `MappingStatistics`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MappingRaw {
    pub total_reads: u64,
    pub mapped_reads: u64,
    pub unmapped_reads: u64,
    pub mapped_reads_frac: f64,
    pub unmapped_reads_frac: f64,
    pub unmapped: UnmappedRaw,
}

/// Mirror of the `unmapped` block of `stats/mapping_map.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnmappedRaw {
    pub missing_probe: u64,
    pub missing_feature: u64,
    pub missing_whitelist: u64,
    pub failed_umi_qual: u64,
    pub umi_truncated: u64,
    pub missing_probe_frac: f64,
    pub missing_feature_frac: f64,
    pub missing_whitelist_frac: f64,
    pub failed_umi_qual_frac: f64,
    pub umi_truncated_frac: f64,
}

/// Mirror of a `stats/mapping_lib.json` entry (`cyto-map` `LibraryStatistics`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryRaw {
    pub name: String,
    pub total_elem: u64,
    pub total_aggr: u64,
    pub total_hash: u64,
    pub position: u64,
    pub mate: String,
    pub window: u64,
    pub exact: bool,
    pub init_time: f64,
}

/// Mirror of a `stats/mapping_run.json` entry (`cyto-map` `InputRuntimeStatistics`).
#[derive(Debug, Clone, Deserialize)]
pub struct RunRaw {
    #[allow(dead_code)]
    pub input_id: u64,
    pub elapsed_sec: f64,
}

/// Mirror of `stats/umi/{probe}.umi.json` (`cyto-ibu-umi-correct` `Statistics`).
#[derive(Debug, Clone, Deserialize)]
pub struct UmiRaw {
    #[allow(dead_code)]
    pub total: u64,
    #[allow(dead_code)]
    pub corrected: u64,
    pub fraction_corrected: f64,
}

/// Mirror of `stats/assignments/{probe}.json` (`geomux --stats`).
#[derive(Debug, Clone, Deserialize)]
pub struct AssignmentRaw {
    #[allow(dead_code)]
    pub n_untested: u64,
    pub n_tested: u64,
    pub n_assigned: u64,
    pub n_unassigned: u64,
    pub dominant_moi: i64,
    pub mois: Vec<i64>,
    pub moi_counts: Vec<u64>,
}

/// A single row of `.timings.tsv`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimingRow {
    pub ibu_name: String,
    pub module: String,
    pub elapsed: f64,
}

/// The kind of library a sample directory holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SampleKind {
    Gex,
    Crispr,
    Unknown,
}

impl SampleKind {
    pub fn label(self) -> &'static str {
        match self {
            SampleKind::Gex => "Gene Expression",
            SampleKind::Crispr => "CRISPR Guide",
            SampleKind::Unknown => "Unknown",
        }
    }
}

/// A single (rank, umis) sample used to draw the barcode-rank knee plot.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct KneePoint {
    pub rank: u64,
    pub umis: u64,
}

/// Guide-assignment summary for a CRISPR probe.
#[derive(Debug, Clone, Serialize)]
pub struct AssignmentSummary {
    pub n_tested: u64,
    pub n_assigned: u64,
    pub n_unassigned: u64,
    pub assigned_frac: f64,
    pub dominant_moi: i64,
    /// (moi, count) pairs, ascending by moi.
    pub moi_distribution: Vec<(i64, u64)>,
}

/// Per-probe (per demultiplexed barcode) summary.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeSummary {
    pub name: String,
    pub n_barcodes: u64,
    pub total_reads: u64,
    pub total_umis: u64,
    /// Sequencing saturation endpoint: `1 - total_umis / total_reads`.
    pub saturation: f64,
    pub median_reads_per_barcode: f64,
    pub median_umis_per_barcode: f64,
    pub umi_corrected_frac: Option<f64>,
    /// Distinct features (genes/guides) with at least one count in this probe's matrix.
    pub features_detected: Option<u64>,
    pub assignment: Option<AssignmentSummary>,
    /// Downsampled points for the barcode-rank plot (not present when reads are absent).
    pub knee: Vec<KneePoint>,
}

/// Everything known about one sample directory.
#[derive(Debug, Clone, Serialize)]
pub struct SampleReport {
    pub sample: String,
    pub path: String,
    pub kind: SampleKind,
    pub mapping: Option<MappingRaw>,
    pub libraries: Vec<LibraryRaw>,
    pub runtime_sec: Option<f64>,
    pub probes: Vec<ProbeSummary>,
    pub timings: Vec<TimingRow>,
    /// Sum of `total_reads` across probes.
    pub total_reads: u64,
    /// Sum of `total_umis` across probes.
    pub total_umis: u64,
    /// Aggregate saturation across probes (`1 - total_umis / total_reads`).
    pub overall_saturation: Option<f64>,
    /// Total features (genes/guides) in the count matrix panel.
    pub features_total: Option<u64>,
    /// Distinct features detected across all probes (union).
    pub features_detected: Option<u64>,
    /// Non-fatal issues encountered while collecting (missing/failed files).
    pub notes: Vec<String>,
}
