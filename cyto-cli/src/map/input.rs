use std::io::Read;

use anyhow::Result;
use clap::Parser;

pub use anyhow::bail;
pub use binseq::MmapReader;
use paraseq::fastq;

use cyto_io::match_input_transparent;

type FqReader = fastq::Reader<Box<dyn Read + Send>>;
type FqReaderPair = (FqReader, FqReader);

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(short = 'i', long, required_unless_present = "input")]
    pub r1: Option<String>,
    #[clap(short = 'I', long, required_unless_present = "input")]
    pub r2: Option<String>,
}
impl PairedInput {
    pub fn to_readers(&self) -> Result<FqReaderPair> {
        let h1 = match_input_transparent(self.r1.as_ref())?;
        let h2 = match_input_transparent(self.r2.as_ref())?;

        let r1 = fastq::Reader::new(h1);
        let r2 = fastq::Reader::new(h2);

        Ok((r1, r2))
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Binseq input options")]
pub struct BinseqInput {
    #[clap(short = 'b', long, conflicts_with_all = ["r1", "r2"])]
    pub input: Option<String>,
}
impl BinseqInput {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_reader(&self) -> Result<MmapReader> {
        if let Some(input) = &self.input {
            let reader = MmapReader::new(input)?;
            Ok(reader)
        } else {
            bail!("No input file provided");
        }
    }
}
