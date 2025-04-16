use super::IbuInput;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct ArgsCorrect {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsCorrect,
}

#[derive(Parser, Debug)]
pub struct OptionsCorrect {
    /// Path of the whitelist file
    ///
    /// This is a file containing a single nucleotide sequence per line
    ///
    /// Compression supported for: [gzip, bzip, lzma, zstd]
    #[clap(short = 'w', long)]
    pub whitelist: String,

    /// Maximum distance from a whitelist sequence to be considered a match.
    ///
    /// Will not accept corrections that are ambiguously distant from multiple whitelist sequences.
    #[clap(short = 'd', long, default_value = "1")]
    pub distance: u32,

    /// Remove ambiguous or non-whitelist sequences
    ///
    /// If this flag is present, sequences that are
    /// not within the distance threshold of a whitelist sequence
    /// or are ambiguously distant from multiple whitelist sequences
    /// will be removed from the output.
    #[clap(short = 'r', long)]
    pub remove: bool,

    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,
}
