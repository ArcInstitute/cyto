use clap::Parser;

use super::{Geometry, PairedInput, RuntimeOptions};
use crate::cli::ArgsOutput;

#[cfg(feature = "binseq")]
use super::BinseqInput;

#[derive(Parser)]
pub struct ArgsGeneric {
    #[clap(flatten)]
    pub input: PairedInput,

    #[cfg(feature = "binseq")]
    #[clap(flatten)]
    pub binseq: BinseqInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub generic: GenericOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser)]
#[clap(next_help_heading = "Generic Options")]
pub struct GenericOptions {
    //// Path to library file
    #[clap(short = 'c', long = "generic")]
    pub generic_filepath: String,

    /// Left-boundary to extract sequence
    #[clap(short = 's', long)]
    pub boundary: usize,

    /// Use exact matching for flex sequences and probes.
    ///
    /// Default allows for unambiguous 1-hamming distance mismatches
    #[clap(short = 'x', long)]
    pub exact_matching: bool,
}
