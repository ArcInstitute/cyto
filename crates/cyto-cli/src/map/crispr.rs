use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::map::{MultiPairedInput, RuntimeOptions};
use crate::ArgsOutput;

use super::MapOptions;

#[derive(Parser, Debug)]
pub struct ArgsCrispr {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub map: MapOptions,

    #[clap(flatten)]
    pub crispr: CrisprOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "CRISPR Options")]
pub struct CrisprOptions {
    /// Path to CRISPR guides library file
    #[clap(short = 'c', long = "guides")]
    pub guides_filepath: String,
}

impl ArgsCrispr {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}
