mod biject;
mod crispr;
mod gex;
mod probe;
mod umi;
mod whitelist;

pub use biject::Bijection;
pub use crispr::CrisprMapper;
pub use gex::GexMapper;
pub use probe::ProbeMapper;
pub use umi::UmiMapper;
pub use whitelist::WhitelistMapper;

use crate::{geometry::ReadMate, stats::LibraryStatistics};

/// Result of a successful feature match against a read sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureMatch {
    /// Index of the matched feature in the library.
    pub feature_idx: usize,
    /// End position (exclusive) of the matched region in the read.
    /// For components with dynamic downstream offsets (e.g. anchor+protospacer),
    /// this is used to compute the actual offset of subsequent components.
    pub end_pos: usize,
}

pub trait Mapper {
    /// Queries the mapper for a feature match in the given sequence.
    fn query(&self, seq: &[u8]) -> Option<FeatureMatch>;

    /// Returns which read (R1/R2) this mapper operates on.
    fn mate(&self) -> ReadMate;
}

impl<T: Mapper + ?Sized> Mapper for Box<T> {
    fn query(&self, seq: &[u8]) -> Option<FeatureMatch> {
        (**self).query(seq)
    }
    fn mate(&self) -> ReadMate {
        (**self).mate()
    }
}

pub trait Library {
    fn statistics(&self) -> LibraryStatistics;
}
impl<T: Library + ?Sized> Library for Box<T> {
    fn statistics(&self) -> LibraryStatistics {
        (**self).statistics()
    }
}

// Typestate markers
pub struct Unpositioned;
pub struct Ready;
