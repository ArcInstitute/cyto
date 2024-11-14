use clap::Parser;

use super::{Geometry, PairedInput};

#[derive(Parser)]
pub struct ArgsBus {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub options: OptionsBus,
}

#[derive(Parser)]
pub struct OptionsBus {
    #[clap(short, long, help = "Output file [default=stdout]")]
    pub output: Option<String>,
}
