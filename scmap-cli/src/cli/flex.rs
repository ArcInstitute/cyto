use super::{ArgsOutput, Geometry, PairedInput, ProbeOptions};
use clap::Parser;

#[derive(Parser)]
pub struct ArgsFlex {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub flex: FlexOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser)]
#[clap(next_help_heading = "FLEX Options")]
pub struct FlexOptions {
    #[clap(short = 'c', long = "flex")]
    pub flex_filepath: String,
    #[clap(short = 's', long, default_value = "16")]
    pub spacer: usize,
}
