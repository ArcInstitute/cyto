use super::IbuInput;

#[derive(clap::Parser)]
pub struct ArgsCount {
    #[clap(flatten)]
    pub input: IbuInput,

    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// Number of threads to use in counting
    #[clap(short = 't', long, default_value = "1")]
    pub num_threads: usize,

    /// Keep the barcode values 2bit compressed in the output
    #[clap(short = 'e', long = "compressed")]
    pub compressed: bool,

    /// File containing the index features
    ///
    /// If this is provided the index features names will be output instead of their index values
    #[clap(short = 'f', long)]
    pub features: Option<String>,
}
