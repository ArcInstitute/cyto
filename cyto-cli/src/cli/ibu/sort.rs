use super::IbuInput;

#[derive(clap::Parser)]
pub struct ArgsSort {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(short, long)]
    pub output: Option<String>,

    #[clap(short = 't', long, default_value = "1")]
    pub num_threads: usize,
}
