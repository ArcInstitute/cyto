mod crispr;
mod geometry;
mod io;

pub type RefNuc<'a> = &'a [u8];

pub use crispr::{Library, Mapper};
pub use geometry::Bus;
pub use io::PairedReader;
