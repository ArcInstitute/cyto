use clap::Parser;

use crate::map::GEOMETRY_GEX_FLEX_V2;

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
    #[clap(long)]
    pub preset: Option<GeometryPreset>,

    /// Path to whitelist file
    #[clap(short = 'w', long)]
    pub whitelist: String,

    /// Path to probe file
    #[clap(short = 'p', long)]
    pub probes: String,

    /// Use exact matching (no hamming distance correction)
    #[clap(short = 'x', long)]
    pub exact: bool,

    /// Remap window size for position adjustment (0 to disable)
    ///
    /// This is the position window size for remapping an element (+/-) on failed match
    ///
    /// If preset to a v2 condition this is ignored and set to 5
    #[clap(long, default_value = "1")]
    remap_window: usize,

    /// Skip UMI quality check
    #[clap(long)]
    pub no_umi_qual_check: bool,

    /// Regex pattern for probe alias
    ///
    /// Used to select/filter probes that are known to be in the dataset
    #[clap(long)]
    pub probe_regex: Option<String>,
}
impl MapOptions {
    pub fn remap_window(&self) -> usize {
        match self.preset {
            Some(GeometryPreset::GexV2) => 5,
            _ => self.remap_window,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum GeometryPreset {
    /// [barcode][umi:12]|[gex][:18][probe]
    GexV1,
    /// [barcode][umi:12][:10][probe]|[gex]
    GexV2,
    /// [barcode][umi:12]|[probe][anchor][protospacer]
    CrisprV1,
    /// [barcode][umi:12]|[:18][probe][anchor][protospacer]
    CrisprProper,
}
impl GeometryPreset {
    pub fn into_geometry_str(&self) -> &str {
        match self {
            Self::GexV1 => GEOMETRY_GEX_FLEX_V1,
            Self::GexV2 => GEOMETRY_GEX_FLEX_V2,
            Self::CrisprV1 => GEOMETRY_CRISPR_FLEX_V1,
            Self::CrisprProper => GEOMETRY_CRISPR_PROPERSEQ,
        }
    }
}
