use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::ArgsOutput;

use super::{Geometry, MapOptions, MultiPairedInput, ProbeOptions, RuntimeOptions};

#[derive(Parser, Debug)]
pub struct ArgsGex {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub gex: GexOptions,

    #[clap(flatten)]
    pub map: MapOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}
impl ArgsGex {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Flex GEX Options")]
pub struct GexOptions {
    //// Path to Flex GEX library file
    #[clap(short = 'c', long = "gex")]
    pub gex_filepath: String,

    /// Spacer sequence length
    #[clap(short = 's', long, default_value = "18")]
    pub spacer: usize,
}
