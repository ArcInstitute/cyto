use clap::Parser;
use cyto::mappers::MapperOffset;

use super::{Geometry, PairedInput, RuntimeOptions};
use crate::cli::ArgsOutput;

#[cfg(feature = "binseq")]
use super::BinseqInput;

#[derive(Parser)]
pub struct ArgsGeneric {
    #[clap(flatten)]
    pub input: PairedInput,

    #[cfg(feature = "binseq")]
    #[clap(flatten)]
    pub binseq: BinseqInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub generic: GenericOptions,

    #[clap(flatten)]
    pub runtime: RuntimeOptions,

    #[clap(flatten)]
    pub output: ArgsOutput,
}

#[derive(Parser)]
#[clap(next_help_heading = "Generic Options")]
pub struct GenericOptions {
    //// Path to library file
    #[clap(short = 'c', long = "generic")]
    pub generic_filepath: String,

    /// Index to extract sequence (right of this point)
    #[clap(
        short = 's',
        long,
        conflicts_with = "left_of",
        required_unless_present = "left_of"
    )]
    pub right_of: Option<usize>,

    /// Index to extract sequence (left of this point)
    #[clap(
        short = 'S',
        long,
        conflicts_with = "right_of",
        required_unless_present = "right_of"
    )]
    pub left_of: Option<usize>,

    /// Use exact matching for flex sequences and probes.
    ///
    /// Default allows for unambiguous 1-hamming distance mismatches
    #[clap(short = 'x', long)]
    pub exact_matching: bool,
}

impl GenericOptions {
    pub fn offset(&self) -> MapperOffset {
        if let Some(right_of) = self.right_of {
            MapperOffset::RightOf(right_of)
        } else if let Some(left_of) = self.left_of {
            MapperOffset::LeftOf(left_of)
        } else {
            unreachable!("This should never happen")
        }
    }
}
