use std::io::Read;

use anyhow::Result;
use binseq::BinseqReader;
use clap::Parser;

pub use anyhow::bail;
pub use binseq::bq::MmapReader;
use log::{debug, error};
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
        match (self.r1.as_ref(), self.r2.as_ref()) {
            (Some(r1), Some(r2)) => {
                debug!("Opening readers for {r1} and {r2}");
                Ok((fastx::Reader::from_path(r1)?, fastx::Reader::from_path(r2)?))
            }
            _ => {
                bail!("Both R1 and R2 must be provided for paired input")
            }
        }
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Paired input options")]
pub struct MultiPairedInput {
    #[clap(short = 'i', conflicts_with = "input", num_args = 1.., required_unless_present = "input")]
    pub pairs: Vec<String>,
}
impl MultiPairedInput {
    pub fn to_readers(&self) -> Result<Vec<FxReaderPair>> {
        let mut readers = Vec::new();
        if self.pairs.len() % 2 != 0 {
            error!(
                "Found {} inputs, expecting an even number of file pairs",
                self.pairs.len()
            );
            bail!("Number of pairs must be even");
        }
        for pair in self.pairs.chunks(2) {
            let r1 = pair[0].clone();
            let r2 = pair[1].clone();
            readers.push((fastx::Reader::from_path(r1)?, fastx::Reader::from_path(r2)?));
        }
        Ok(readers)
    }
}

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Binseq input options")]
pub struct BinseqInput {
    #[clap(
        short = 'b',
        long,
        conflicts_with = "pairs",
        required_unless_present = "pairs"
    )]
    pub input: Option<String>,
}
impl BinseqInput {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_reader(&self) -> Result<BinseqReader> {
        let path = self.path()?;
        debug!("Opening binseq reader for {path}");
        let rdr = BinseqReader::new(path)?;
        if !rdr.is_paired() {
            error!("Found unpaired BINSEQ file: {path}");
            bail!("Input BINSEQ file must be paired!");
        }
        Ok(rdr)
    }

    pub fn path(&self) -> Result<&str> {
        if let Some(input) = &self.input {
            Ok(input)
        } else {
            bail!("No input file provided to BINSEQ input");
        }
    }
}
