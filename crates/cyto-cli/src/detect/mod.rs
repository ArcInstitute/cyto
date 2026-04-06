use clap::{Parser, Subcommand};

use crate::map::{CrisprOptions, GexOptions, MultiPairedInput, ProbeOptions, WhitelistOptions};

#[derive(Subcommand, Debug)]
pub enum DetectCommand {
    /// Auto-detect geometry for a Flex GEX library
    Gex(ArgsDetectGex),
    /// Auto-detect geometry for a Flex CRISPR library
    Crispr(ArgsDetectCrispr),
}

#[derive(Parser, Debug)]
pub struct ArgsDetectGex {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub whitelist: WhitelistOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub gex: GexOptions,

    #[clap(flatten)]
    pub detection: DetectionOptions,
}

#[derive(Parser, Debug)]
pub struct ArgsDetectCrispr {
    #[clap(flatten)]
    pub input: MultiPairedInput,

    #[clap(flatten)]
    pub whitelist: WhitelistOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub crispr: CrisprOptions,

    #[clap(flatten)]
    pub detection: DetectionOptions,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Detection Options")]
pub struct DetectionOptions {
    /// Number of reads to sample for geometry detection
    #[clap(long, default_value = "10000")]
    pub num_reads: usize,

    /// Minimum proportion of reads matching a component to accept it
    #[clap(long, default_value = "0.10")]
    pub min_proportion: f64,

    /// Minimum proportion of reads at a position for remap window estimation
    ///
    /// Positions with fewer matches than this proportion are treated as noise.
    #[clap(long, default_value = "0.01")]
    pub remap_min_proportion: f64,
}
