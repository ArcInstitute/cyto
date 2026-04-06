use clap::Parser;

use crate::map::{GEOMETRY_CRISPR_FLEX_V2, GEOMETRY_GEX_FLEX_V2};

use super::{GEOMETRY_CRISPR_FLEX_V1, GEOMETRY_CRISPR_PROPERSEQ, GEOMETRY_GEX_FLEX_V1};

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Mapping Options")]
pub struct MapOptions {
    /// Custom Geometry DSL string
    ///
    /// If unsure, try a preset first.
    #[clap(short = 'g', long)]
    pub geometry: Option<String>,

    /// Geometry Preset
    ///
    /// Required unless a custom geometry is provided.
    #[clap(long)]
    pub preset: Option<GeometryPreset>,

    /// Remap window size for position adjustment (0 to disable)
    ///
    /// This is the position window size for remapping an element (+/-) on failed match
    ///
    /// If preset to a v2 condition this is ignored and set to 5
    #[clap(long, default_value = "1", conflicts_with = "preset")]
    remap_window: usize,

    /// Number of reads to sample for geometry auto-detection (0 to disable)
    #[clap(long, default_value = "10000")]
    pub geometry_auto_num_reads: usize,

    /// Minimum proportion of reads matching auto-detected geometry
    #[clap(long, default_value = "0.10")]
    pub geometry_auto_min_proportion: f64,

    /// Minimum proportion of reads at a position for remap window estimation
    #[clap(long, default_value = "0.01")]
    pub geometry_auto_remap_min_proportion: f64,

    /// Use exact matching (no hamming distance correction)
    #[clap(short = 'x', long)]
    pub exact: bool,

    /// Skip UMI quality check
    #[clap(long)]
    pub no_umi_qual_check: bool,

    #[clap(flatten)]
    whitelist: WhitelistOptions,

    #[clap(flatten)]
    probe: ProbeOptions,
}
impl MapOptions {
    pub fn remap_window(&self) -> usize {
        match self.preset {
            Some(GeometryPreset::GexV2 | GeometryPreset::CrisprV2) => 5,
            _ => self.remap_window,
        }
    }

    pub fn probe_regex(&self) -> Option<&str> {
        self.probe.probe_regex.as_deref()
    }

    pub fn probe_path(&self) -> Option<&str> {
        self.probe.probes.as_deref()
    }

    pub fn whitelist_path(&self) -> &str {
        &self.whitelist.whitelist
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Probe Options")]
pub struct ProbeOptions {
    /// Path to probe file
    #[clap(short = 'p', long)]
    pub probes: Option<String>,

    /// Regex pattern for probe alias
    ///
    /// Used to select/filter probes that are known to be in the dataset
    #[clap(long)]
    pub probe_regex: Option<String>,
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Whitelist Options")]
pub struct WhitelistOptions {
    /// Path to whitelist file
    #[clap(short = 'w', long)]
    pub whitelist: String,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum GeometryPreset {
    /// [barcode][umi:12]|[gex][:18][probe]
    GexV1,
    /// [barcode][umi:12][:10][probe]|[gex]
    GexV2,
    /// [barcode][umi:12]|[probe][anchor][protospacer]
    CrisprV1,
    /// [barcode][umi:12][:10][probe]|[:14][anchor][protospacer]
    CrisprV2,
    /// [barcode][umi:12]|[:18][probe][anchor][protospacer]
    CrisprProper,
}
impl GeometryPreset {
    pub fn into_geometry_str(&self) -> &str {
        match self {
            Self::GexV1 => GEOMETRY_GEX_FLEX_V1,
            Self::GexV2 => GEOMETRY_GEX_FLEX_V2,
            Self::CrisprV1 => GEOMETRY_CRISPR_FLEX_V1,
            Self::CrisprV2 => GEOMETRY_CRISPR_FLEX_V2,
            Self::CrisprProper => GEOMETRY_CRISPR_PROPERSEQ,
        }
    }
}
