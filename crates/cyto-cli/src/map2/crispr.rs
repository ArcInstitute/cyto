use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::map::{MultiPairedInput, RuntimeOptions};
use crate::ArgsOutput;

use super::Map2Options;

#[derive(Parser, Debug)]
pub struct ArgsCrispr2 {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub map2: Map2Options,

    #[clap(flatten)]
    pub crispr: Crispr2Options,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "CRISPR Options")]
pub struct Crispr2Options {
    /// Path to CRISPR guides library file
    #[clap(short = 'c', long = "guides")]
    pub guides_filepath: String,
}

impl ArgsCrispr2 {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}
