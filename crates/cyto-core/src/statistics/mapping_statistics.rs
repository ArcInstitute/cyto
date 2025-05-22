use anyhow::Result;
use serde::Serialize;
use std::{io::Write, ops::Add};

use super::ProcessedMappingStatistics;
use crate::mappers::MappingError;

#[derive(Debug, Default, Serialize, Clone, Copy)]
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
    pub fn process(&self) -> ProcessedMappingStatistics {
        self.into()
    }
    pub fn save_json<W: Write>(&self, writer: W) -> Result<()> {
        self.process().save_json(writer)
    }
    pub fn merge(&mut self, other: &Self) {
        *self = *self + *other;
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
impl Add for MappingStatistics {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            total_reads: self.total_reads + other.total_reads,
            mapped_reads: self.mapped_reads + other.mapped_reads,
            unmapped_reads: self.unmapped_reads + other.unmapped_reads,
            mapping_errors: self.mapping_errors + other.mapping_errors,
        }
    }
}

#[derive(Debug, Default, Serialize, Clone, Copy)]
pub struct MappingErrorStatistics {
    pub missing_flex_sequence: usize,
    pub missing_anchor: usize,
    pub missing_protospacer: usize,
    pub missing_probe: usize,
    pub missing_target_sequence: usize,
}
impl MappingErrorStatistics {
    pub fn increment(&mut self, error: MappingError) {
        match error {
            MappingError::MissingFlexSequence => self.missing_flex_sequence += 1,
            MappingError::MissingAnchor => self.missing_anchor += 1,
            MappingError::MissingProtospacer => self.missing_protospacer += 1,
            MappingError::MissingProbe => self.missing_probe += 1,
            MappingError::MissingTargetSequence => self.missing_target_sequence += 1,
        }
    }
}
impl Add for MappingErrorStatistics {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            missing_flex_sequence: self.missing_flex_sequence + other.missing_flex_sequence,
            missing_anchor: self.missing_anchor + other.missing_anchor,
            missing_protospacer: self.missing_protospacer + other.missing_protospacer,
            missing_probe: self.missing_probe + other.missing_probe,
            missing_target_sequence: self.missing_target_sequence + other.missing_target_sequence,
        }
    }
}
