mod cli;
mod commands;
mod io;
mod progress;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Crispr(args) => commands::crispr::run(args),
        Commands::Flex(args) => commands::flex::run(args),
    }
}
