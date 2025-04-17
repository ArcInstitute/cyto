use super::IbuInput;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct ArgsUmi {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsCorrect,
}

#[derive(Parser, Debug)]
pub struct OptionsCorrect {
    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,
}
