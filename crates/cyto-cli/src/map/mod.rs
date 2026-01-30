pub mod crispr;
pub mod gex;
mod input;
mod options;
mod runtime;

pub use crispr::ArgsCrispr;
pub use gex::ArgsGex;
pub use input::MultiPairedInput;
pub use options::MapOptions;
pub use runtime::RuntimeOptions;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum MapCommand {
    /// Map sequences to a Flex GEX library (v2)
    Gex(ArgsGex),
    /// Map sequences to a Flex CRISPR library (v2)
    Crispr(ArgsCrispr),
}

impl MapCommand {
    pub fn validate_outdir(&self) -> Result<()> {
        match self {
            MapCommand::Gex(args) => args.output.validate_outdir(),
            MapCommand::Crispr(args) => args.output.validate_outdir(),
        }
    }

    pub fn log_path(&self) -> PathBuf {
        match self {
            MapCommand::Gex(args) => args.output.log_path(),
            MapCommand::Crispr(args) => args.output.log_path(),
        }
    }
}

pub const GEOMETRY_GEX_FLEX_V1: &str = "[barcode][umi:12] | [gex][:18][probe]";
pub const GEOMETRY_GEX_FLEX_V2: &str = "[barcode][umi:12][:10][probe] | [gex]";
pub const GEOMETRY_CRISPR_FLEX_V1: &str = "[barcode][umi:12] | [probe][anchor][protospacer]";
pub const GEOMETRY_CRISPR_FLEX_V2: &str =
    "[barcode][umi:12][:10][probe] | [:14][anchor][protospacer]";
pub const GEOMETRY_CRISPR_PROPERSEQ: &str = "[barcode][umi:12] | [:18][probe][anchor][protospacer]";
