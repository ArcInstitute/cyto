use anyhow::Result;
use cyto_cli::{Commands, IbuCommand, MapCommand, WorkflowCommand};
use log::info;

use clap::{
    Parser,
    builder::{
        Styles,
        styling::{AnsiColor, Effects},
    },
};

mod logging;
use crate::logging::{setup_default_logging, setup_workflow_logging};

// Configures Clap v3-style help menu colors
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Yellow.on_default());

#[derive(Parser)]
#[command(styles = STYLES)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}
impl Cli {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::parse()
    }
}

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
        Commands::Map(map) => match map {
            MapCommand::Gex(args) => cyto_map::run_gex2(&args),
            MapCommand::Crispr(args) => cyto_map::run_crispr2(&args),
        },
        Commands::Ibu(ibu) => match ibu {
            IbuCommand::View(args) => cyto_ibu_view::run(&args),
            IbuCommand::Sort(args) => cyto_ibu_sort::run(&args),
            IbuCommand::Count(args) => cyto_ibu_count::run(&args),
            IbuCommand::Cat(args) => cyto_ibu_cat::run(&args),
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
