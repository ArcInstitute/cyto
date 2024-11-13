mod counters;
pub mod crispr;
pub mod flex;
mod geometry;
pub mod io;
pub mod probe;

pub type RefNuc<'a> = &'a [u8];

pub use counters::{BusCounter, ProbeBusCounter};
pub use geometry::Bus;
pub use io::PairedReader;
