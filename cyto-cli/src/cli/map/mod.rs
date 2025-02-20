pub mod crispr;
pub mod flex;
pub mod generic;
pub mod geometry;
pub mod input;
pub mod map;
pub mod probe;
pub mod runtime;

pub use crispr::ArgsCrispr;
pub use flex::ArgsFlex;
pub use generic::ArgsGeneric;
pub use geometry::Geometry;
pub use input::PairedInput;
pub use map::MapOptions;
pub use probe::ProbeOptions;
pub use runtime::RuntimeOptions;

#[cfg(feature = "binseq")]
use input::BinseqInput;

use clap::Subcommand;

#[derive(Subcommand)]
/// Map sequences to a library
pub enum MapCommand {
    /// Map sequences to a CRISPR library
    Crispr(ArgsCrispr),
    /// Map sequences to a FLEX library
    Flex(ArgsFlex),
    /// Map sequences to a generic library
    Generic(ArgsGeneric),
}
