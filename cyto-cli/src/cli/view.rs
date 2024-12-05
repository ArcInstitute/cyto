use clap::Parser;

use super::{Geometry, PairedInput};

#[derive(Parser)]
pub struct ArgsView {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub options: OptionsView,
}

#[derive(Parser)]
pub struct OptionsView {
    #[clap(short, long, help = "Output file [default=stdout]")]
    pub output: Option<String>,
}
