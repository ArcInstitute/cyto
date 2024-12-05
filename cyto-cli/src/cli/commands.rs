use clap::Subcommand;

use super::{ArgsView, MapCommand};

#[derive(Subcommand)]
pub enum Commands {
    /// Map sequences to a library
    #[clap(subcommand)]
    Map(MapCommand),

    /// Separate the Barcode, UMI, and R2 sequence and view as plain text
    View(ArgsView),
}
