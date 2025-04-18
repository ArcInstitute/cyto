use clap::Subcommand;

use super::{ArgsView, IbuCommand, MapCommand, WorkflowCommand};

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

    /// Separate the Barcode, UMI, and R2 sequence and view as plain text
    View(ArgsView),
}
