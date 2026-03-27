use clap::Subcommand;

use super::{ArgsDownload, IbuCommand, MapCommand, WorkflowCommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Executes a common workflow
    #[clap(subcommand)]
    Workflow(WorkflowCommand),

    /// Map sequences to a library
    #[clap(subcommand)]
    Map(MapCommand),

    /// Perform operations on an IBU library
    #[clap(subcommand)]
    Ibu(IbuCommand),

    /// Download reference resources to ~/.cyto/
    Download(ArgsDownload),
}
