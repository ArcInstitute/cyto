use clap::Parser;

use super::{Geometry, MapOptions, PairedInput, ProbeOptions, RuntimeOptions};
use crate::cli::ArgsOutput;

#[cfg(feature = "binseq")]
use super::BinseqInput;

#[derive(Parser)]
pub struct ArgsFlex {
    #[clap(flatten)]
    pub input: PairedInput,

    #[cfg(feature = "binseq")]
    #[clap(flatten)]
    pub binseq: BinseqInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub flex: FlexOptions,

    #[clap(flatten)]
    pub map: MapOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser)]
#[clap(next_help_heading = "FLEX Options")]
pub struct FlexOptions {
    //// Path to flex GEX library file
    #[clap(short = 'c', long = "flex")]
    pub flex_filepath: String,

    /// Spacer sequence length
    #[clap(short = 's', long, default_value = "18")]
    pub spacer: usize,
}
