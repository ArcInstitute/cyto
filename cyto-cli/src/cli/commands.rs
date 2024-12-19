use clap::Subcommand;

use super::{ArgsView, IbuCommand, MapCommand};

#[derive(Subcommand)]
pub enum Commands {
    /// Map sequences to a library
    #[clap(subcommand)]
    Map(MapCommand),

    /// Perform operations on an IBU library
    #[clap(subcommand)]
    Ibu(IbuCommand),

    /// Separate the Barcode, UMI, and R2 sequence and view as plain text
    View(ArgsView),
}
