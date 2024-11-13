use super::ArgsCrispr;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Crispr(ArgsCrispr),
}
