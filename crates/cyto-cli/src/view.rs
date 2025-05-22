use clap::Parser;

use super::{Geometry, PairedInput};

#[derive(Parser, Debug)]
pub struct ArgsView {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub options: OptionsView,
}

#[derive(Parser, Debug)]
pub struct OptionsView {
    /// Number of threads to use
    #[clap(short = 'T', long, default_value = "1")]
    pub threads: usize,

    #[clap(short, long, help = "Output file [default=stdout]")]
    pub output: Option<String>,
}
