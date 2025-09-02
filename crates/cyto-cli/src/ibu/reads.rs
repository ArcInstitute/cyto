use crate::ibu::IbuInput;

#[derive(clap::Parser, Debug)]
pub struct ArgsReads {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsReads,
}

#[derive(clap::Parser, Debug)]
pub struct OptionsReads {
    /// Output file path to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// Keep the barcode as an encoded u64
    #[clap(long)]
    pub encoded: bool,

    /// Do not write a header to the output file
    #[clap(long)]
    pub no_header: bool,
}
