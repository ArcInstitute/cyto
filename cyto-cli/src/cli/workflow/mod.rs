use clap::{Parser, Subcommand};

use super::{ArgsCrispr, ArgsFlex};

#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// Executes a flex mapping workflow
    FlexMapping(FlexMappingCommand),

    /// Executes a crispr mapping workflow
    CrisprMapping(CrisprMappingCommand),
}

#[derive(Parser, Debug)]
pub struct FlexMappingCommand {
    #[clap(flatten)]
    pub flex_args: ArgsFlex,
}

#[derive(Parser, Debug)]
pub struct CrisprMappingCommand {
    #[clap(flatten)]
    pub crispr_args: ArgsCrispr,
}
