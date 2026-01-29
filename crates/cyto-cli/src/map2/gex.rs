use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::map::{MultiPairedInput, RuntimeOptions};
use crate::ArgsOutput;

use super::Map2Options;

#[derive(Parser, Debug)]
pub struct ArgsGex2 {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub map2: Map2Options,

    #[clap(flatten)]
    pub gex: Gex2Options,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "GEX Options")]
pub struct Gex2Options {
    /// Path to GEX library file
    #[clap(short = 'c', long = "gex")]
    pub gex_filepath: String,
}

impl ArgsGex2 {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}
