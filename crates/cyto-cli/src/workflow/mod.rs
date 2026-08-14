use std::{fmt::Display, path::PathBuf, process::Command};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use log::{debug, error};

use crate::{ArgsCrispr, ArgsGex};

pub const VERSION_GEOMUX: &str = "0.5.5";
pub const VERSION_CELL_FILTER: &str = "0.1.2";
pub const VERSION_PYCYTO: &str = "0.1.14";

#[derive(Subcommand, Debug)]
pub enum WorkflowCommand {
    /// Executes a gex mapping workflow (map => sort => umi-correct => count => filter)
    #[clap(name = "gex")]
    GexMapping(GexMappingCommand),

    /// Executes a crispr mapping workflow (map => sort => umi-correct => count => assign)
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
    pub geomux_args: ArgsGeomux,

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
    /// Skip UMI correction step
    #[clap(long)]
    pub skip_umi: bool,

    /// Skip reads/umi saturation step
    #[clap(long)]
    pub skip_reads: bool,

    /// Skip `EmptyDrops` filtering step (GEX)
    ///
    /// Only used when format is h5ad
    #[clap(long)]
    pub no_filter: bool,

    /// Keep the unfiltered h5ad file (GEX)
    ///
    /// Only used when format is h5ad
    #[clap(long)]
    pub keep_unfiltered: bool,

    /// Keep the IBU file(s) after counting
    #[clap(long)]
    pub keep_ibu: bool,

    /// Skip CRISPR-barcode assignment step (CRISPR)
    ///
    /// Only used when format is h5ad
    #[clap(long)]
    pub skip_assignment: bool,

    /// Sort in memory instead of using disk
    #[clap(long)]
    pub sort_in_memory: bool,

    /// Memory limit for sorting (ignored if `sort_in_memory` is true)
    #[clap(long, default_value = "5GiB")]
    pub memory_limit: String,

    #[clap(short = 'F', long, default_value = "h5ad")]
    pub format: CountFormat,
}
impl ArgsWorkflow {
    pub fn validate_requirements(&self, mode: WorkflowMode) -> Result<()> {
        // The external Python tools run only when converting to h5ad;
        // `cyto-workflow`'s `ibu_steps` gates convert/filter/assign on
        // `to_h5ad()`. Keep these guards in sync with those invocation sites so
        // we never resolve a tool the run will not use (nothing in mtx/tsv mode).
        if !self.to_h5ad() {
            return Ok(());
        }

        debug!("Checking if `uvx` exists in $PATH");
        match Command::new("uvx").args(["--version"]).output() {
            Ok(_) => debug!("Found `uvx` in $PATH"),
            Err(e) => {
                error!("Encountered an unexpected error checking for `uvx`: {e}");
                bail!("Encountered an unexpected error checking for `uvx`: {e}");
            }
        }
        warm_uvx("pycyto", VERSION_PYCYTO)?;
        if mode == WorkflowMode::Gex && !self.no_filter {
            warm_uvx("cell-filter", VERSION_CELL_FILTER)?;
        }
        if mode == WorkflowMode::Crispr && !self.skip_assignment {
            warm_uvx("geomux", VERSION_GEOMUX)?;
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

/// Build a `uvx --from '{name}=={version}' {name}` command that runs the pinned
/// tool in an ephemeral, isolated environment. The caller appends the tool's own
/// subcommand and arguments. Never mutates the user's global `uv tool` set.
pub fn uvx_command(name: &str, version: &str) -> Command {
    let mut cmd = Command::new("uvx");
    cmd.arg("--from")
        .arg(format!("{name}=={version}"))
        .arg(name);
    cmd
}

/// Pre-resolve and cache the pinned tool's ephemeral `uvx` environment so the
/// first real invocation (inside the parallel per-probe section) does not pay a
/// cold resolve, and so a resolution failure surfaces up front.
fn warm_uvx(name: &str, version: &str) -> Result<()> {
    debug!("Resolving `{name}=={version}` via uvx if necessary...");
    match uvx_command(name, version).arg("--help").output() {
        Ok(output) if output.status.success() => {
            debug!("Resolved `{name}`");
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to resolve `{name}` via uvx: {stderr}");
            bail!("Failed to resolve `{name}` via uvx: {stderr}");
        }
        Err(e) => {
            error!("Encountered an unexpected error resolving `{name}` via uvx: {e}");
            bail!("Encountered an unexpected error resolving `{name}` via uvx: {e}");
        }
    }
}

#[derive(Parser, Debug, Clone, Copy)]
#[clap(next_help_heading = "Geomux Options")]
pub struct ArgsGeomux {
    /// Minimum number of UMIs required for a cell to be included in geomux testing.
    ///
    /// 5 for geomux
    /// 3 for mixture
    #[clap(long)]
    geomux_min_umi_cells: Option<usize>,
    /// Minimum number of UMIs required for a guide to be included in geomux testing.
    #[clap(long, default_value_t = 5)]
    pub geomux_min_umi_guides: usize,
    /// Log odds ratio minimum threshold to use for geomux assignments.
    #[clap(long)]
    pub geomux_log_odds_ratio: Option<f64>,
    /// fdr threshold to use for geomux assignments.
    #[clap(long, default_value_t = 0.05)]
    pub geomux_fdr_threshold: f64,
    /// Mode to use for geomux testing.
    #[clap(long, default_value = "geomux")]
    pub geomux_mode: GeomuxMode,
}
impl ArgsGeomux {
    pub fn min_umi_cells(&self) -> usize {
        self.geomux_min_umi_cells.unwrap_or(match self.geomux_mode {
            GeomuxMode::Geomux => 5,
            GeomuxMode::Mixture => 3,
        })
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum GeomuxMode {
    /// Use the hypergeometric test.
    Geomux,
    /// Use the gaussian mixture model
    Mixture,
}
impl Display for GeomuxMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeomuxMode::Geomux => write!(f, "geomux"),
            GeomuxMode::Mixture => write!(f, "mixture"),
        }
    }
}
