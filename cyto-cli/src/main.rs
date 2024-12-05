mod cli;
mod commands;
mod io;
mod progress;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, MapCommand};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::View(args) => commands::view::run(args),
        Commands::Map(map) => match map {
            MapCommand::Crispr(args) => commands::crispr::run(args),
            MapCommand::Flex(args) => commands::flex::run(args),
        },
        Commands::Ibu(ibu) => match ibu {},
    }
}
