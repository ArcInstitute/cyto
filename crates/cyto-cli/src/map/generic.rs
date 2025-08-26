use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use cyto_core::mappers::MapperOffset;

use super::{Geometry, MapOptions, PairedInput, RuntimeOptions};
use crate::ArgsOutput;

use super::BinseqInput;

#[derive(Parser, Debug)]
pub struct ArgsGeneric {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub binseq: BinseqInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub generic: GenericOptions,

    #[clap(flatten)]
    pub map: MapOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}
impl ArgsGeneric {
    pub fn validate_outdir(&self) -> Result<()> {
        self.output.validate_outdir()
    }
    pub fn log_path(&self) -> PathBuf {
        self.output.log_path()
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Generic Options")]
pub struct GenericOptions {
    //// Path to library file
    #[clap(short = 'c', long = "generic")]
    pub generic_filepath: String,

    /// Index to extract sequence (right of this point)
    #[clap(short = 's', long, conflicts_with = "left_of")]
    pub right_of: Option<usize>,

    /// Index to extract sequence (left of this point)
    #[clap(short = 'S', long, conflicts_with = "right_of")]
    pub left_of: Option<usize>,
}

impl GenericOptions {
    pub fn offset(&self) -> Option<MapperOffset> {
        if let Some(right_of) = self.right_of {
            Some(MapperOffset::RightOf(right_of))
        } else {
            self.left_of.map(MapperOffset::LeftOf)
        }
    }
}
