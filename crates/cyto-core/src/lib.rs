pub mod aliases;
mod counters;
pub mod deduplicate;
mod geometry;
pub mod io;
pub mod libraries;
pub mod mappers;
pub mod metadata;
pub mod statistics;

pub use counters::{
    BarcodeIndexCounter, BusCounter, Counter, ProbeBarcodeIndexCounter, ProbeBusCounter,
};
pub use deduplicate::{deduplicate_umis, BarcodeIndexCount, BarcodeIndexCounts, DeduplicateError};
pub use geometry::{Bus, GeometryR1};
pub use mappers::Mapper;
pub use statistics::MappingStatistics;
