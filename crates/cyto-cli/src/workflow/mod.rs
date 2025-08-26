use std::{path::PathBuf, process::Command};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use log::{debug, error};

use super::{ArgsCrispr, ArgsGex};

pub const VERSION_GEOMUX: &str = "0.4.4";
pub const VERSION_CELL_FILTER: &str = "0.1.1";
pub const VERSION_PYCYTO: &str = "0.1.0";

#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// Executes a gex mapping workflow (map => sort => barcode => sort => umi => sort => count)
    #[clap(name = "gex")]
    GexMapping(GexMappingCommand),

    /// Executes a crispr mapping workflow (map => sort => barcode => sort => umi => sort => count)
    #[clap(name = "crispr")]
    CrisprMapping(CrisprMappingCommand),
}
impl WorkflowCommand {
    pub fn validate_outdir(&self) -> Result<()> {
        match self {
            WorkflowCommand::GexMapping(cmd) => cmd.gex_args.validate_outdir(),
            WorkflowCommand::CrisprMapping(cmd) => cmd.crispr_args.validate_outdir(),
        }
    }

    pub fn log_path(&self) -> PathBuf {
        match self {
            WorkflowCommand::GexMapping(cmd) => cmd.gex_args.log_path(),
            WorkflowCommand::CrisprMapping(cmd) => cmd.crispr_args.log_path(),
        }
    }
}

#[derive(Parser, Debug)]
pub struct GexMappingCommand {
    #[clap(flatten)]
    pub gex_args: ArgsGex,

    #[clap(flatten)]
    pub wf_args: ArgsWorkflow,
}
impl GexMappingCommand {
    pub fn mode(&self) -> WorkflowMode {
        WorkflowMode::Gex
    }
}

#[derive(Parser, Debug)]
pub struct CrisprMappingCommand {
    #[clap(flatten)]
    pub crispr_args: ArgsCrispr,

    #[clap(flatten)]
    pub wf_args: ArgsWorkflow,
}
impl CrisprMappingCommand {
    pub fn mode(&self) -> WorkflowMode {
        WorkflowMode::Crispr
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowMode {
    Gex,
    Crispr,
}
impl WorkflowMode {
    pub fn should_filter(&self) -> bool {
        match self {
            WorkflowMode::Gex => true,
            WorkflowMode::Crispr => false,
        }
    }
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

    /// Skip EmptyDrops filtering step (GEX)
    ///
    /// Only used when format is h5ad
    #[clap(long)]
    pub no_filter: bool,

    /// Keep the unfiltered h5ad file (GEX)
    ///
    /// Only used when format is h5ad
    #[clap(long)]
    pub keep_unfiltered: bool,

    /// Skip CRISPR-barcode assignment step (CRISPR)
    ///
    /// Only used when format is h5ad
    #[clap(long)]
    pub skip_assignment: bool,

    /// Cell Barcode Whitelist
    #[clap(short, long, required_unless_present = "skip_barcode")]
    pub whitelist: String,

    #[clap(short = 'F', long, default_value = "h5ad")]
    pub format: CountFormat,
}
impl ArgsWorkflow {
    pub fn validate_requirements(&self, mode: WorkflowMode) -> Result<()> {
        if self.format == CountFormat::H5ad || !self.no_filter {
            debug!("Checking if `uv` exists in $PATH");
            match Command::new("uv").args(["--version"]).output() {
                Ok(_) => debug!("Found `uv` in $PATH"),
                Err(e) => {
                    error!("Encountered an unexpected error checking for `uv`: {e}");
                    bail!("Encountered an unexpected error checking for `uv`: {}", e);
                }
            }
            transparent_uv_install("pycyto", VERSION_PYCYTO)?;
        }
        if mode == WorkflowMode::Gex && !self.no_filter {
            transparent_uv_install("cell-filter", VERSION_CELL_FILTER)?;
        }
        if mode == WorkflowMode::Crispr {
            transparent_uv_install("geomux", VERSION_GEOMUX)?;
        }
        Ok(())
    }

    /// Check whether the workflow should output mtx files
    ///
    /// This is true if the format is mtx or h5ad but mtx is consumed by h5ad
    pub fn mtx(&self) -> bool {
        match self.format {
            CountFormat::H5ad | CountFormat::Mtx => true,
            CountFormat::Tsv => false,
        }
    }

    /// Check whether the workflow should output h5ad files
    pub fn to_h5ad(&self) -> bool {
        match self.format {
            CountFormat::H5ad => true,
            CountFormat::Mtx | CountFormat::Tsv => false,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, clap::ValueEnum, PartialEq, Eq)]
pub enum CountFormat {
    #[default]
    H5ad,
    Mtx,
    Tsv,
}

fn transparent_uv_install(name: &str, version: &str) -> Result<()> {
    debug!("Installing `{}@{}` if necessary...", name, version);
    match Command::new("uv")
        .arg("tool")
        .arg("install")
        .arg(format!("{}@{}", name, version))
        .output()
    {
        Ok(_) => {
            debug!("Precompiling `{}`...", name);
            match Command::new(name).arg("--help").output() {
                Ok(_) => {
                    debug!("Precompiled `{}`", name);
                    Ok(())
                }
                Err(e) => {
                    error!(
                        "Encountered an unexpected error precompiling `{}`: {e}",
                        name
                    );
                    bail!(
                        "Encountered an unexpected error precompiling `{}`: {}",
                        name,
                        e
                    );
                }
            }
        }
        Err(e) => {
            error!("Encountered an unexpected error installing `{}`: {e}", name);
            bail!(
                "Encountered an unexpected error installing `{}`: {}",
                name,
                e
            );
        }
    }
}
