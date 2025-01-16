pub mod crispr;
pub mod flex;
pub mod geometry;
pub mod input;
pub mod probe;

pub use crispr::ArgsCrispr;
pub use flex::ArgsFlex;
pub use geometry::Geometry;
pub use input::{BinseqInput, PairedInput};
pub use probe::ProbeOptions;

use clap::Subcommand;

#[derive(Subcommand)]
/// Map sequences to a library
pub enum MapCommand {
    /// Map sequences to a CRISPR library
    Crispr(ArgsCrispr),
    /// Map sequences to a FLEX library
    Flex(ArgsFlex),
}
