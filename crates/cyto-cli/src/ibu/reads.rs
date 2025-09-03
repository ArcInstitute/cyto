use std::path::Path;

use crate::ibu::IbuInput;

#[derive(clap::Parser, Debug)]
pub struct ArgsReads {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsReads,
}
impl ArgsReads {
    pub fn from_wf_path<P: AsRef<Path>>(input_path: &str, output_path: P) -> Self {
        let outpath = output_path.as_ref().to_string_lossy().to_string();
        Self {
            input: IbuInput::from_path(input_path),
            options: OptionsReads {
                output: Some(outpath),
                whitelist: None,
                encoded: false,
                no_header: false,
            },
        }
    }
}

#[derive(clap::Parser, Debug)]
pub struct OptionsReads {
    /// Output file path to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// A whitelist of barcodes to keep [default=all]
    #[clap(short, long)]
    pub whitelist: Option<String>,

    /// Keep the barcode as an encoded u64
    #[clap(long)]
    pub encoded: bool,

    /// Do not write a header to the output file
    #[clap(long)]
    pub no_header: bool,
}
