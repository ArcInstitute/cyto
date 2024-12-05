use super::{ArgsOutput, Geometry, PairedInput, ProbeOptions};
use clap::Parser;

#[derive(Parser)]
pub struct ArgsCrispr {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub crispr: CrisprOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser)]
#[clap(next_help_heading = "CRISPR Options")]
pub struct CrisprOptions {
    #[clap(short = 'c', long = "guides")]
    pub guides_filepath: String,
    #[clap(short = 's', long, default_value = "26")]
    pub offset: usize,
}
