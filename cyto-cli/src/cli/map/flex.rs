use clap::Parser;

use super::{BinseqInput, Geometry, PairedInput, ProbeOptions, RuntimeOptions};
use crate::cli::ArgsOutput;

#[derive(Parser)]
pub struct ArgsFlex {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub binseq: BinseqInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub flex: FlexOptions,

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

    /// Use exact matching for flex sequences and probes.
    ///
    /// Default allows for unambiguous 1-hamming distance mismatches
    #[clap(short = 'x', long)]
    pub exact_matching: bool,
}
