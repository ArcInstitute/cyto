use super::IbuInput;

#[derive(clap::Parser, Debug)]
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

    /// The column in the feature table to aggregate reads over - skips aggregation if this is zero
    #[clap(short = 'C', long, default_value_t = 1)]
    pub feature_col: usize,
}
impl ArgsCount {
    pub fn from_wf_path(
        sort_path: &str,
        out_path: &str,
        features_path: &str,
        num_threads: usize,
    ) -> Self {
        Self {
            input: IbuInput::from_path(sort_path),
            output: Some(out_path.to_string()),
            features: Some(features_path.to_string()),
            compressed: false,
            feature_col: 1,
            num_threads,
        }
    }
}
