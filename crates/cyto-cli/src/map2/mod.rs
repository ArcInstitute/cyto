pub mod crispr;
pub mod gex;
mod options;

pub use crispr::ArgsCrispr2;
pub use gex::ArgsGex2;
pub use options::Map2Options;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Map2Command {
    /// Map sequences to a Flex GEX library (v2)
    Gex(ArgsGex2),
    /// Map sequences to a Flex CRISPR library (v2)
    Crispr(ArgsCrispr2),
}

impl Map2Command {
    pub fn validate_outdir(&self) -> Result<()> {
        match self {
            Map2Command::Gex(args) => args.output.validate_outdir(),
            Map2Command::Crispr(args) => args.output.validate_outdir(),
        }
    }

    pub fn log_path(&self) -> PathBuf {
        match self {
            Map2Command::Gex(args) => args.output.log_path(),
            Map2Command::Crispr(args) => args.output.log_path(),
        }
    }
}

pub const GEOMETRY_GEX_FLEX_V1: &str = "[barcode][umi:12] | [gex][:18][probe]";
pub const GEOMETRY_GEX_FLEX_V2: &str = "[barcode][umi:12][:10][probe] | [gex]";
pub const GEOMETRY_CRISPR_FLEX_V1: &str = "[barcode][umi:12] | [probe][anchor][protospacer]";
pub const GEOMETRY_CRISPR_FLEX_V2: &str =
    "[barcode][umi:12][:10][probe] | [:14][anchor][protospacer]";
pub const GEOMETRY_CRISPR_PROPERSEQ: &str = "[barcode][umi:12] | [:18][probe][anchor][protospacer]";
