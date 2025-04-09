use super::IbuInput;

#[derive(clap::Parser, Debug)]
pub struct ArgsSort {
    #[clap(flatten)]
    pub input: IbuInput,

    /// Output file to write to
    ///
    /// Required unless `-p/--pipe` is present.
    #[clap(short, long, required_unless_present("pipe"))]
    pub output: Option<String>,

    /// Pipe the output to stdout
    ///
    /// Due to binary output, this flag is necessary not to flood the terminal with binary.
    #[clap(short, long, conflicts_with("output"))]
    pub pipe: bool,

    #[clap(short = 't', long, default_value = "1")]
    pub num_threads: usize,
}
