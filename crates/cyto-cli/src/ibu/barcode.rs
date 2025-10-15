use std::path::Path;

use super::IbuInput;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct ArgsBarcode {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsBarcode,
}
impl ArgsBarcode {
    pub fn from_wf_path<P: AsRef<Path>>(
        input_path: &str,
        output_path: &str,
        whitelist: &str,
        bc_log: P,
        exact: bool,
        skip_second_pass: bool,
    ) -> Self {
        Self {
            input: IbuInput::from_path(input_path),
            options: OptionsBarcode {
                whitelist: whitelist.to_string(),
                exact,
                skip_second_pass,
                include: false,
                output: Some(output_path.to_string()),
                log: Some(bc_log.as_ref().display().to_string()),
            },
        }
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Barcode Correction Options")]
pub struct OptionsBarcode {
    /// Path of the whitelist file
    ///
    /// This is a file containing a single nucleotide sequence per line
    ///
    /// Compression supported for: [gzip, bzip, lzma, zstd]
    #[clap(short = 'w', long)]
    pub whitelist: String,

    /// Exact match only
    ///
    /// If this flag is present, only exact matches will be accepted.
    #[clap(long = "bc-exact")]
    pub exact: bool,

    /// Skip second pass correction
    ///
    /// Second pass correction is enabled by default and collapses cell barcodes into the maximum abundance ambiguous parent
    #[clap(short = 's', long)]
    pub skip_second_pass: bool,

    /// Include ambiguous or non-whitelist sequences
    ///
    /// If this flag is present, sequences that are
    /// not within the distance threshold of a whitelist sequence
    /// or are ambiguously distant from multiple whitelist sequences
    /// will be included in the output.
    #[clap(short = 'I', long)]
    pub include: bool,

    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// Output file to write statistics to [default=stderr]
    ///
    /// Will output as json
    #[clap(short, long)]
    pub log: Option<String>,
}
