use anyhow::Result;
use serde::Serialize;
use std::io::Write;

use super::{mapping_statistics::MappingErrorStatistics, MappingStatistics};

/// Calculate the fraction of two integers, returning 0.0 if the denominator is 0.
fn fraction(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[derive(Debug, Default, Serialize, Clone, Copy)]
pub struct ProcessedMappingStatistics {
    pub total_reads: usize,
    pub mapped_reads: usize,
    pub unmapped_reads: usize,
    pub fraction_mapped: f64,
    pub fraction_unmapped: f64,
    pub mapping_errors: ProcessedMappingErrorStatistics,
}
impl From<MappingStatistics> for ProcessedMappingStatistics {
    fn from(ms: MappingStatistics) -> Self {
        ProcessedMappingStatistics {
            total_reads: ms.total_reads,
            mapped_reads: ms.mapped_reads,
            unmapped_reads: ms.unmapped_reads,
            fraction_mapped: fraction(ms.mapped_reads, ms.total_reads),
            fraction_unmapped: fraction(ms.unmapped_reads, ms.total_reads),
            mapping_errors: ProcessedMappingErrorStatistics::new(
                ms.mapping_errors,
                ms.unmapped_reads,
            ),
        }
    }
}
impl From<&MappingStatistics> for ProcessedMappingStatistics {
    fn from(ms: &MappingStatistics) -> Self {
        (*ms).into()
    }
}
impl ProcessedMappingStatistics {
    pub fn save_json<W: Write>(&self, writer: W) -> Result<()> {
        Ok(serde_json::to_writer_pretty(writer, self)?)
    }
}

#[derive(Debug, Default, Serialize, Clone, Copy)]
pub struct ProcessedMappingErrorStatistics {
    pub missing_gex_sequence: usize,
    pub missing_anchor: usize,
    pub missing_protospacer: usize,
    pub missing_probe: usize,

    pub fraction_missing_gex_sequence: f64,
    pub fraction_missing_anchor: f64,
    pub fraction_missing_protospacer: f64,
    pub fraction_missing_probe: f64,
}
impl ProcessedMappingErrorStatistics {
    pub fn new(mes: MappingErrorStatistics, unmapped_reads: usize) -> Self {
        Self {
            missing_gex_sequence: mes.missing_gex_sequence,
            missing_anchor: mes.missing_anchor,
            missing_protospacer: mes.missing_protospacer,
            missing_probe: mes.missing_probe,

            fraction_missing_gex_sequence: fraction(mes.missing_gex_sequence, unmapped_reads),
            fraction_missing_anchor: fraction(mes.missing_anchor, unmapped_reads),
            fraction_missing_protospacer: fraction(mes.missing_protospacer, unmapped_reads),
            fraction_missing_probe: fraction(mes.missing_probe, unmapped_reads),
        }
    }
}
