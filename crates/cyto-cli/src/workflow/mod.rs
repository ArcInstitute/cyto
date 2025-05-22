use clap::{Parser, Subcommand};

use super::{ArgsCrispr, ArgsFlex};

#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// Executes a flex mapping workflow (map => sort => barcode => sort => umi => sort => count)
    #[clap(name = "flex")]
    FlexMapping(FlexMappingCommand),

    /// Executes a crispr mapping workflow (map => sort => barcode => sort => umi => sort => count)
    #[clap(name = "crispr")]
    CrisprMapping(CrisprMappingCommand),
}

#[derive(Parser, Debug)]
pub struct FlexMappingCommand {
    #[clap(flatten)]
    pub flex_args: ArgsFlex,

    #[clap(flatten)]
    pub wf_args: ArgsWorkflow,
}

#[derive(Parser, Debug)]
pub struct CrisprMappingCommand {
    #[clap(flatten)]
    pub crispr_args: ArgsCrispr,

    #[clap(flatten)]
    pub wf_args: ArgsWorkflow,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Workflow Options")]
pub struct ArgsWorkflow {
    /// Skip barcode correction step
    #[clap(long)]
    pub skip_barcode: bool,

    /// Skip UMI correction step
    #[clap(long)]
    pub skip_umi: bool,

    /// Cell Barcode Whitelist
    #[clap(short, long, required_unless_present = "skip_barcode")]
    pub whitelist: String,
}
