use std::path::Path;

use super::IbuInput;

#[derive(clap::Parser, Debug)]
pub struct ArgsCount {
    #[clap(flatten)]
    pub input: IbuInput,

    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// Output mtx format.
    /// Will treat `output` as a directory and create 3 files:
    ///
    /// (1) barcodes.txt.gz
    /// (2) features.txt.gz
    /// (3) matrix.mtx.gz
    #[clap(long, requires = "output", requires = "features")]
    pub mtx: bool,

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
    pub fn from_wf_path<P: AsRef<Path>>(
        sort_path: &str,
        out_path: P,
        features_path: P,
        num_threads: usize,
        mtx: bool,
    ) -> Self {
        Self {
            input: IbuInput::from_path(sort_path),
            output: Some(out_path.as_ref().to_str().unwrap().to_string()),
            features: Some(features_path.as_ref().to_str().unwrap().to_string()),
            mtx,
            compressed: false,
            feature_col: 1,
            num_threads,
        }
    }
}
