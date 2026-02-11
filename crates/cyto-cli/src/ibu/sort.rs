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

    /// Memory limit for sorting in-memory
    #[clap(short, long, default_value = "5GiB")]
    pub memory_limit: String,

    /// Perform the sorting in-memory [default: on-disk merge sort]
    ///
    /// This may be faster for small datasets, but will load IBUs fully into memory.
    #[clap(long)]
    pub in_memory: bool,

    /// Pipe the output to stdout
    ///
    /// Due to binary output, this flag is necessary not to flood the terminal with binary.
    #[clap(short, long, conflicts_with("output"))]
    pub pipe: bool,

    #[clap(short = 't', long, default_value = "1")]
    pub num_threads: usize,
}
impl ArgsSort {
    pub fn from_wf_path(
        path: &str,
        output: &str,
        in_memory: bool,
        memory_limit: String,
        num_threads: usize,
    ) -> Self {
        let input = IbuInput::from_path(path);
        Self {
            input,
            num_threads,
            output: Some(output.to_string()),
            pipe: false,
            in_memory,
            memory_limit,
        }
    }
}
