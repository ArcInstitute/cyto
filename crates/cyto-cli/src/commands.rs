use clap::Subcommand;

use super::{ArgsDownload, ArgsSummary, DetectCommand, IbuCommand, MapCommand, WorkflowCommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Executes a common workflow
    #[clap(subcommand)]
    Workflow(WorkflowCommand),

    /// Map sequences to a library
    #[clap(subcommand)]
    Map(MapCommand),

    /// Detect read geometry from input files and print it to stdout
    #[clap(subcommand)]
    Detect(DetectCommand),

    /// Perform operations on an IBU library
    #[clap(subcommand)]
    Ibu(IbuCommand),

    /// Generate HTML QC reports from finished output directories
    Summary(ArgsSummary),

    /// Download reference resources to ~/.cyto/
    Download(ArgsDownload),
}
