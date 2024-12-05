use clap::Subcommand;

use super::{ArgsCrispr, ArgsFlex};

#[derive(Subcommand)]
/// Map sequences to a library
pub enum MapCommand {
    /// Map sequences to a CRISPR library
    Crispr(ArgsCrispr),
    /// Map sequences to a FLEX library
    Flex(ArgsFlex),
}
