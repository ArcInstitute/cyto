use super::{ArgsCrispr, ArgsFlex};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Crispr(ArgsCrispr),
    Flex(ArgsFlex),
}
