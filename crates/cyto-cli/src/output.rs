use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use cyto_io::validate_output_directory;

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Output Options")]
pub struct ArgsOutput {
    /// Output directory path
    #[clap(short = 'o', long, default_value = "./cyto_out")]
    pub outdir: String,

    /// Force overwrite of existing output directory
    #[clap(short = 'f', long)]
    pub force: bool,

    /// Minimum number of records required to keep an IBU file
    ///
    /// IBU files with fewer records than this threshold will be removed.
    /// A value of 0 disables the filter (only truly empty files are removed).
    ///
    /// This is useful when you have a large number of possible probes but are only expecting a smaller number of observed probes.
    /// If you want to specify probes see the `--probe-regex` flag.
    #[clap(long, default_value_t = 1_000)]
    pub min_ibu_records: u64,
}
impl ArgsOutput {
    pub fn validate_outdir(&self) -> Result<()> {
        validate_output_directory(&self.outdir, self.force)
    }
    pub fn log_path(&self) -> PathBuf {
        PathBuf::from(&self.outdir).join("cyto.log")
    }
}
