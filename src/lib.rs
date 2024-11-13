mod geometry;
mod io;

pub type RefNuc<'a> = &'a [u8];

pub use geometry::Bus;
pub use io::PairedReader;
