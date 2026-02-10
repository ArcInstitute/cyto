use std::path::Path;

use anyhow::Result;
use cyto_io::open_file_handle;
use derive_more::AddAssign;
use log::debug;

use crate::ReadMate;

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct InputRuntimeStatistics {
    /// The input identity
    pub input_id: usize,
    /// The amount of time taken to map the input (seconds)
    pub elapsed_sec: f64,
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct LibraryStatistics {
    /// The name of the library type
    pub name: &'static str,

    /// The total number of elements in the library
    pub total_elem: usize,

    /// The number of aggregated elements in the library
    ///
    /// This can vary in meaning depending on the library type.
    /// For GEX - it's the number of genes
    /// For Probes - it's the number of demultiplexing probes (not their sequences)
    pub total_aggr: usize,

    /// The total number of hash keys in this library
    pub total_hash: usize,

    /// The mapped position of this library in the query constructs
    pub position: usize,

    /// The read mate searched for this library
    pub mate: ReadMate,

    /// The remap window size used for this library
    pub window: usize,

    /// Whether this library used exact matching
    pub exact: bool,

    /// The amount of time taken to initialize the library (seconds)
    pub init_time: f64,
}

/// Aggregate mapping statistics for a library
///
/// Note: fraction values should not be trusted on accumulated, they are recalculated on `with_fractions`
#[derive(Clone, Copy, Default, Debug, AddAssign, serde::Serialize)]
pub struct MappingStatistics {
    pub total_reads: usize,
    pub mapped_reads: usize,
    pub unmapped_reads: usize,
    pub mapped_reads_frac: f64,
    pub unmapped_reads_frac: f64,
    pub unmapped: UnmappedStatistics,
}
impl MappingStatistics {
    /// Calculate the percentage mapped
    pub fn frac_mapped(&self) -> f64 {
        self.mapped_reads as f64 / self.total_reads as f64
    }
    /// Resets all counters on the statistics
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    /// Prepare statistics with computed fractions for output
    pub fn with_fractions(self) -> Self {
        let total_unmapped = self.total_reads - self.mapped_reads;
        Self {
            total_reads: self.total_reads,
            mapped_reads: self.mapped_reads,
            unmapped_reads: total_unmapped,
            mapped_reads_frac: self.mapped_reads as f64 / self.total_reads as f64,
            unmapped_reads_frac: total_unmapped as f64 / self.total_reads as f64,
            unmapped: self.unmapped.with_fractions(total_unmapped),
        }
    }
}

/// Aggregate mapping statistics for a library focused on unmapped reads
///
/// Note: fraction values should not be trusted from accumulated, they are recalculated on `with_fractions`
#[derive(Clone, Copy, Default, Debug, AddAssign, serde::Serialize)]
pub struct UnmappedStatistics {
    pub missing_probe: usize,
    pub missing_feature: usize,
    pub missing_whitelist: usize,
    pub failed_umi_qual: usize,
    pub umi_truncated: usize,
    pub missing_probe_frac: f64,
    pub missing_feature_frac: f64,
    pub missing_whitelist_frac: f64,
    pub failed_umi_qual_frac: f64,
    pub umi_truncated_frac: f64,
}

impl UnmappedStatistics {
    /// Calculate fractions based on the total unmapped reads
    pub fn with_fractions(self, total_unmapped: usize) -> Self {
        let total = total_unmapped as f64;
        // simple function to handle saturated division
        let frac = |x: usize| {
            if total > 0.0 { x as f64 / total } else { 0.0 }
        };
        Self {
            missing_probe: self.missing_probe,
            missing_feature: self.missing_feature,
            missing_whitelist: self.missing_whitelist,
            failed_umi_qual: self.failed_umi_qual,
            umi_truncated: self.umi_truncated,
            missing_probe_frac: frac(self.missing_probe),
            missing_feature_frac: frac(self.missing_feature),
            missing_whitelist_frac: frac(self.missing_whitelist),
            failed_umi_qual_frac: frac(self.failed_umi_qual),
            umi_truncated_frac: frac(self.umi_truncated),
        }
    }
}

pub fn write_statistics<P: AsRef<Path>>(
    outdir: P,
    libstats: &[LibraryStatistics],
    mapstats: MappingStatistics,
    runstats: &[InputRuntimeStatistics],
) -> Result<()> {
    let stats_outdir = outdir.as_ref().join("stats");

    impl_write_statistics(stats_outdir.join("mapping_lib.json"), libstats)?;
    impl_write_statistics(
        stats_outdir.join("mapping_map.json"),
        mapstats.with_fractions(),
    )?;
    impl_write_statistics(stats_outdir.join("mapping_run.json"), runstats)?;
    Ok(())
}

fn impl_write_statistics<P: AsRef<Path>, S: serde::Serialize>(path: P, stat: S) -> Result<()> {
    debug!("Saving statistics to: {}", path.as_ref().display());
    let mut handle = open_file_handle(&path)?;
    serde_json::to_writer_pretty(&mut handle, &stat)?;
    Ok(())
}
