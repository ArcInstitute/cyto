use super::IbuInput;

#[derive(clap::Parser)]
pub struct ArgsCount {
    #[clap(flatten)]
    pub input: IbuInput,

    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    #[clap(short = 't', long, default_value = "1")]
    pub num_threads: usize,
}
