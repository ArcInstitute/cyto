use anyhow::Result;
use serde::Serialize;
use std::io::Write;

use super::MappingError;

#[derive(Debug, Default, Serialize)]
pub struct MappingStatistics {
    pub total_reads: usize,
    pub mapped_reads: usize,
    pub unmapped_reads: usize,
    pub mapping_errors: MappingErrorStatistics,
}
impl MappingStatistics {
    pub fn increment_mapped(&mut self) {
        self.total_reads += 1;
        self.mapped_reads += 1;
    }
    pub fn increment_unmapped(&mut self, why: MappingError) {
        self.total_reads += 1;
        self.unmapped_reads += 1;
        self.mapping_errors.increment(why);
    }
    pub fn increment_unmapped_multi_reason(&mut self, why1: MappingError, why2: MappingError) {
        self.total_reads += 1;
        self.unmapped_reads += 1;
        self.mapping_errors.increment(why1);
        self.mapping_errors.increment(why2);
    }
    pub fn save_json<W: Write>(&self, writer: W) -> Result<()> {
        Ok(serde_json::to_writer(writer, self)?)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct MappingErrorStatistics {
    missing_flex_sequence: usize,
    missing_anchor: usize,
    missing_protospacer: usize,
    missing_probe: usize,
}
impl MappingErrorStatistics {
    pub fn increment(&mut self, error: MappingError) {
        match error {
            MappingError::MissingFlexSequence => self.missing_flex_sequence += 1,
            MappingError::MissingAnchor => self.missing_anchor += 1,
            MappingError::MissingProtospacer => self.missing_protospacer += 1,
            MappingError::MissingProbe => self.missing_probe += 1,
        }
    }
}
