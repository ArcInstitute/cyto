pub mod crispr;
mod geometry;
mod io;
pub mod probe;

pub type RefNuc<'a> = &'a [u8];

pub use geometry::Bus;
pub use io::PairedReader;
