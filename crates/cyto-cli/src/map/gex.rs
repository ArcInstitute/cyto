use clap::Parser;

use crate::ArgsOutput;

use super::{BinseqInput, Geometry, MapOptions, PairedInput, ProbeOptions, RuntimeOptions};

#[derive(Parser, Debug)]
pub struct ArgsGex {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub binseq: BinseqInput,

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
