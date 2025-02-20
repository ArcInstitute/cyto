use clap::Parser;

use super::{Geometry, MapOptions, PairedInput, ProbeOptions, RuntimeOptions};
use crate::cli::ArgsOutput;

#[cfg(feature = "binseq")]
use super::BinseqInput;

#[derive(Parser)]
pub struct ArgsCrispr {
    #[clap(flatten)]
    pub input: PairedInput,

    #[cfg(feature = "binseq")]
    #[clap(flatten)]
    pub binseq: BinseqInput,

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

#[derive(Parser)]
#[clap(next_help_heading = "CRISPR Options")]
pub struct CrisprOptions {
    /// Path to CRISPR library
    #[clap(short = 'c', long = "guides")]
    pub guides_filepath: String,

    /// Offset for anchor sequences
    #[clap(short = 's', long, default_value = "26")]
    pub offset: usize,
}
