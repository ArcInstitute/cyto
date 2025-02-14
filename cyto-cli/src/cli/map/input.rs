use std::io::Read;

use anyhow::Result;
use clap::Parser;

#[cfg(feature = "binseq")]
pub use anyhow::bail;
#[cfg(feature = "binseq")]
pub use binseq::PairedMmapReader;
use paraseq::fastq;

use crate::io::match_input_transparent;

#[derive(Parser)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(short = 'i', long, required_unless_present = "input")]
    pub r1: Option<String>,
    #[clap(short = 'I', long, required_unless_present = "input")]
    pub r2: Option<String>,
}
impl PairedInput {
    pub fn to_readers(
        &self,
    ) -> Result<(
        fastq::Reader<Box<dyn Read + Send>>,
        fastq::Reader<Box<dyn Read + Send>>,
    )> {
        let h1 = match_input_transparent(self.r1.as_ref())?;
        let h2 = match_input_transparent(self.r2.as_ref())?;

        let r1 = fastq::Reader::new(h1);
        let r2 = fastq::Reader::new(h2);

        Ok((r1, r2))
    }
}

#[cfg(feature = "binseq")]
#[derive(Parser)]
#[clap(next_help_heading = "Binseq input options")]
pub struct BinseqInput {
    #[clap(short = 'b', long, conflicts_with_all = ["r1", "r2"])]
    pub input: Option<String>,
}
#[cfg(feature = "binseq")]
impl BinseqInput {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_reader(&self) -> Result<PairedMmapReader> {
        if let Some(input) = &self.input {
            PairedMmapReader::new(input)
        } else {
            bail!("No input file provided");
        }
    }
}
