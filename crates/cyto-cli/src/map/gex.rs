use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::map::{MultiPairedInput, RuntimeOptions};
use crate::ArgsOutput;

use super::MapOptions;

#[derive(Parser, Debug)]
pub struct ArgsGex {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub map: MapOptions,

    #[clap(flatten)]
    pub gex: GexOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "GEX Options")]
pub struct GexOptions {
    /// Path to GEX library file
    #[clap(short = 'c', long = "gex")]
    pub gex_filepath: String,
}

impl ArgsGex {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}
