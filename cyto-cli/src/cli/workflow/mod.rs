use clap::{Parser, Subcommand};

use super::ArgsFlex;

#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// Executes a flex mapping workflow
    FlexMapping(FlexMappingCommand),
}

#[derive(Parser, Debug)]
pub struct FlexMappingCommand {
    #[clap(flatten)]
    pub flex_args: ArgsFlex,
}
