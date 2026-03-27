use std::path::PathBuf;

use clap::Args;

#[derive(Args, Debug)]
pub struct ArgsDownload {
    /// Output directory for downloaded resources [default: ~/.cyto/]
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// Re-download even if resources already exist
    #[clap(short, long)]
    pub force: bool,

    /// Download resources for a specific version instead of the current binary version
    #[clap(short, long)]
    pub version: Option<String>,

    /// Override the download URL (useful for testing or custom mirrors)
    #[clap(long, hide = true)]
    pub url: Option<String>,
}
