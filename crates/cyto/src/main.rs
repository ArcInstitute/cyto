use anyhow::Result;
use cyto_cli::{Cli, Commands, IbuCommand, MapCommand, WorkflowCommand};
use log::info;

mod logging;
use crate::logging::{setup_default_logging, setup_workflow_logging};

fn main() -> Result<()> {
    let args = Cli::new();

    match &args.command {
        Commands::Map(map) => {
            map.validate_outdir()?;
            setup_workflow_logging(map.log_path())?;
        }
        Commands::Workflow(wf) => {
            wf.validate_outdir()?;
            setup_workflow_logging(wf.log_path())?;
        }
        _ => setup_default_logging(),
    }

    info!("Initializing...");
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
            IbuCommand::Reads(args) => cyto_ibu_reads::run(&args),
        },
        Commands::Workflow(workflow) => match workflow {
            WorkflowCommand::GexMapping(args) => cyto_workflow::gex::run(&args),
            WorkflowCommand::CrisprMapping(args) => cyto_workflow::crispr::run(&args),
        },
    }?;
    info!("Done!");

    Ok(())
}
