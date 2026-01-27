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
    /// Paths to input files to process.
    ///
    /// If using BINSEQ input (*.bq/*.vbq), the ordering of files or number of files does not matter.
    ///
    /// If using FASTX input, the input files are expected to be paired and sequentially ordered (`S1_R1`, `S1_R2`, `S2_R1`, `S2_R2`, ...).
    /// This is expecting an even number of files.
    #[clap(num_args = 1.., required=true)]
    pub inputs: Vec<String>,
}
impl MultiPairedInput {
    pub fn is_binseq(&self) -> bool {
        self.inputs
            .iter()
            .all(|path| path.ends_with(".bq") || path.ends_with(".vbq") || path.ends_with("cbq"))
    }

    pub fn to_fx_readers(&self) -> Result<Vec<FxReaderPair>> {
        let mut readers = Vec::new();
        if !self.inputs.len().is_multiple_of(2) {
            error!(
                "Found {} inputs, expecting an even number of file pairs",
                self.inputs.len()
            );
            bail!("Number of pairs must be even");
        }
        for pair in self.inputs.chunks(2) {
            let r1 = pair[0].clone();
            let r2 = pair[1].clone();
            readers.push((fastx::Reader::from_path(r1)?, fastx::Reader::from_path(r2)?));
        }
        Ok(readers)
    }

    pub fn to_binseq_readers(&self) -> Result<Vec<BinseqReader>> {
        let mut readers = Vec::new();
        for path in &self.inputs {
            let reader = BinseqReader::new(path)?;
            readers.push(reader);
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
