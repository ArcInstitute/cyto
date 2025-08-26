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
pub use input::{BinseqInput, MultiPairedInput, PairedInput};
pub use map::MapOptions;
pub use probe::ProbeOptions;
pub use runtime::RuntimeOptions;

use std::path::PathBuf;

use anyhow::Result;
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
impl MapCommand {
    pub fn validate_outdir(&self) -> Result<()> {
        match self {
            MapCommand::Crispr(args) => args.validate_outdir(),
            MapCommand::Gex(args) => args.validate_outdir(),
            MapCommand::Generic(args) => args.validate_outdir(),
        }
    }
    pub fn log_path(&self) -> PathBuf {
        match self {
            MapCommand::Crispr(args) => args.log_path(),
            MapCommand::Gex(args) => args.log_path(),
            MapCommand::Generic(args) => args.log_path(),
        }
    }
}
