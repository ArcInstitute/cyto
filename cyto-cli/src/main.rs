mod cli;
mod commands;
mod io;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, IbuCommand, MapCommand, WorkflowCommand};
use commands::{ibu as ibu_commands, map as map_commands, workflow as workflow_commands};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::View(args) => commands::view::run(&args),
        Commands::Map(map) => match map {
            MapCommand::Crispr(args) => map_commands::crispr::run(&args),
            MapCommand::Flex(args) => map_commands::flex::run(&args),
            MapCommand::Generic(args) => map_commands::generic::run(&args),
        },
        Commands::Ibu(ibu) => match ibu {
            IbuCommand::View(args) => ibu_commands::view::run(&args),
            IbuCommand::Sort(args) => ibu_commands::sort::run(&args),
            IbuCommand::Count(args) => ibu_commands::count::run(&args),
            IbuCommand::Correct(args) => ibu_commands::correct::run(&args),
        },
        Commands::Workflow(workflow) => match workflow {
            WorkflowCommand::FlexMapping(args) => workflow_commands::flex::run(&args),
            WorkflowCommand::CrisprMapping(args) => workflow_commands::crispr::run(&args),
        },
    }
}
