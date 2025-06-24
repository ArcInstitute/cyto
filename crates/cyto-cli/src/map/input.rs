use std::io::Read;

use anyhow::Result;
use clap::Parser;

pub use anyhow::bail;
pub use binseq::bq::MmapReader;
use paraseq::fastx;

type FxReader = fastx::Reader<Box<dyn Read + Send>>;
type FxReaderPair = (FxReader, FxReader);

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(
        short = 'i',
        long,
        conflicts_with = "input",
        required_unless_present = "input"
    )]
    pub r1: Option<String>,
    #[clap(
        short = 'I',
        long,
        conflicts_with = "input",
        required_unless_present = "input"
    )]
    pub r2: Option<String>,
}
impl PairedInput {
    pub fn to_readers(&self) -> Result<FxReaderPair> {
        Ok((
            fastx::Reader::from_optional_path(self.r1.as_ref())?,
            fastx::Reader::from_optional_path(self.r2.as_ref())?,
        ))
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Binseq input options")]
pub struct BinseqInput {
    #[clap(short = 'b', long, conflicts_with_all = ["r1", "r2"], required_unless_present_all = ["r1", "r2"])]
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
