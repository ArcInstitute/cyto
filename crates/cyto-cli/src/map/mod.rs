pub mod crispr;
pub mod generic;
pub mod geometry;
pub mod gex;
pub mod input;
pub mod map;
pub mod probe;
pub mod runtime;

pub use crispr::ArgsCrispr;
pub use generic::ArgsGeneric;
pub use geometry::Geometry;
pub use gex::ArgsGex;
pub use input::PairedInput;
pub use map::MapOptions;
pub use probe::ProbeOptions;
pub use runtime::RuntimeOptions;

use input::BinseqInput;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
/// Map sequences to a library
pub enum MapCommand {
    /// Map sequences to a Flex CRISPR library
    Crispr(ArgsCrispr),
    /// Map sequences to a Flex GEX library
    Gex(ArgsGex),
    /// Map sequences to a generic library
    Generic(ArgsGeneric),
}
