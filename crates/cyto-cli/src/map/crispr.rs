use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use super::{Geometry, MapOptions, MultiPairedInput, ProbeOptions, RuntimeOptions};
use crate::ArgsOutput;

#[derive(Parser, Debug)]
pub struct ArgsCrispr {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub crispr: CrisprOptions,

    #[clap(flatten)]
    pub map: MapOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}
impl ArgsCrispr {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "CRISPR Options")]
pub struct CrisprOptions {
    /// Path to CRISPR library
    #[clap(short = 'c', long = "guides")]
    pub guides_filepath: String,

    /// Offset for anchor sequences
    #[clap(short = 's', long, default_value = "26")]
    pub offset: usize,

    /// Lookback size for probe sequences
    ///
    /// This will skip back `n` bases from the start of the anchor sequence to match
    /// the right-hand side of the probe sequence.
    ///
    /// [probe][lookback-size][anchor]
    #[clap(short = 'l', long, default_value_t = 0)]
    pub lookback: usize,
}
