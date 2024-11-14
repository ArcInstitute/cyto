use clap::Subcommand;

use super::{ArgsBus, ArgsCrispr, ArgsFlex};

#[derive(Subcommand)]
pub enum Commands {
    Crispr(ArgsCrispr),
    Flex(ArgsFlex),
    Bus(ArgsBus),
}
