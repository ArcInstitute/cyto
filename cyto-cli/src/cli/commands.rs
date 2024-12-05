use clap::Subcommand;

use super::{ArgsBus, MapCommand};

#[derive(Subcommand)]
pub enum Commands {
    #[clap(subcommand)]
    Map(MapCommand),
    // Crispr(ArgsCrispr),
    // Flex(ArgsFlex),
    Bus(ArgsBus),
}
