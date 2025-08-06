use std::process::Command;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use log::{debug, error};

use super::{ArgsCrispr, ArgsGex};

#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// Executes a gex mapping workflow (map => sort => barcode => sort => umi => sort => count)
    #[clap(name = "gex")]
    GexMapping(GexMappingCommand),

    /// Executes a crispr mapping workflow (map => sort => barcode => sort => umi => sort => count)
    #[clap(name = "crispr")]
    CrisprMapping(CrisprMappingCommand),
}

#[derive(Parser, Debug)]
pub struct GexMappingCommand {
    #[clap(flatten)]
    pub gex_args: ArgsGex,

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

    /// Output counts as MTX
    #[clap(long, conflicts_with = "h5ad")]
    mtx: bool,

    /// Output counts as H5AD
    #[clap(long, conflicts_with = "mtx")]
    pub h5ad: bool,
}
impl ArgsWorkflow {
    pub fn validate_requirements(&self) -> Result<()> {
        if self.h5ad {
            debug!("Checking if `uv` exists in path");
            match Command::new("uv").args(["--version"]).output() {
                Ok(_) => debug!("Found `uv` in $PATH"),
                Err(e) => {
                    error!("Encountered an unexpected error checking for `uv`: {}", e);
                    bail!("Encountered an unexpected error checking for `uv`: {}", e);
                }
            }
        }
        Ok(())
    }

    pub fn mtx(&self) -> bool {
        // MTX is enabled if either flag is set
        self.mtx || self.h5ad
    }
}
