use super::IbuInput;

#[derive(clap::Parser)]
pub struct ArgsSort {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(short, long)]
    pub output: Option<String>,
}
