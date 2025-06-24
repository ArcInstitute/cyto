use super::IbuInput;

#[derive(clap::Parser, Debug)]
pub struct ArgsView {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsView,
}

#[derive(clap::Parser, Debug)]
pub struct OptionsView {
    /// Decode the contents of the IBU library (from 2bit)
    #[clap(short, long)]
    pub decode: bool,

    /// Only output the header of the IBU library
    #[clap(short = 'H', long, conflicts_with = "skip_header")]
    pub header: bool,

    /// Skip outputting the header of the IBU library
    ///
    /// Be careful when doing this if not decoding the library as you
    /// may not be able to decode correctly without the header
    #[clap(short = 'S', long)]
    pub skip_header: bool,

    /// File containing the index features
    ///
    /// If this is provided the index features names will be output instead of their index values
    #[clap(short = 'f', long)]
    pub features: Option<String>,

    /// The column in the feature file to use (the unit name, aggr name, etc.)
    #[clap(short = 'C', long, default_value_t = 0)]
    pub feature_col: usize,

    /// Output file [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,
}
