mod counters;
pub mod crispr;
mod geometry;
pub mod io;
pub mod probe;

pub type RefNuc<'a> = &'a [u8];

pub use counters::BusCounter;
pub use geometry::Bus;
pub use io::{BusCounterWriter, PairedReader};
