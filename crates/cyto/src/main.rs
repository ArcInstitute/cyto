use std::{env, str::FromStr};

use log::{LevelFilter, info};

use anyhow::Result;
use cyto_cli::{Cli, Commands, IbuCommand, MapCommand, WorkflowCommand};

fn main() -> Result<()> {
    let log_level = if let Ok(var) = env::var("CYTO_LOG_LEVEL") {
        LevelFilter::from_str(&var).unwrap_or(LevelFilter::Info)
    } else {
        LevelFilter::Info
    };

    env_logger::builder()
        .format_timestamp_secs()
        .filter_level(log_level)
        .init();

    info!("Initializing cyto");

    let args = Cli::new();
    match args.command {
        Commands::View(args) => cyto_view::run(&args),
        Commands::Map(map) => match map {
            MapCommand::Crispr(args) => cyto_map::crispr::run(&args),
            MapCommand::Gex(args) => cyto_map::gex::run(&args),
            MapCommand::Generic(args) => cyto_map::generic::run(&args),
        },
        Commands::Ibu(ibu) => match ibu {
            IbuCommand::View(args) => cyto_ibu_view::run(&args),
            IbuCommand::Sort(args) => cyto_ibu_sort::run(&args),
            IbuCommand::Count(args) => cyto_ibu_count::run(&args),
            IbuCommand::Cat(args) => cyto_ibu_cat::run(&args),
            IbuCommand::Barcode(args) => cyto_ibu_barcode_correct::run(&args),
            IbuCommand::Umi(args) => cyto_ibu_umi_correct::run(&args),
        },
        Commands::Workflow(workflow) => match workflow {
            WorkflowCommand::GexMapping(args) => cyto_workflow::gex::run(&args),
            WorkflowCommand::CrisprMapping(args) => cyto_workflow::crispr::run(&args),
        },
    }
}
