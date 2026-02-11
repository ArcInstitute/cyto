use anyhow::{Context, Result};
use binseq::BinseqReader;
use clap::Parser;

pub use anyhow::bail;
use log::error;
use paraseq::{fastx, BoxedReader};

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

    pub fn to_binseq_readers(&self) -> Result<Vec<BinseqReader>> {
        let mut readers = Vec::new();
        for path in &self.inputs {
            let reader = BinseqReader::new(path).context("Failed to open BINSEQ reader")?;
            if !reader.is_paired() {
                error!(
                    "Provided BINSEQ path is not paired. All inputs are expected to be paired: {path}"
                );
                bail!("Input file is not paired: {path}");
            }
            readers.push(reader);
        }
        Ok(readers)
    }

    pub fn to_paraseq_collection(&self) -> Result<fastx::Collection<BoxedReader>> {
        if !self.inputs.len().is_multiple_of(2) {
            error!(
                "Found {} inputs, expecting an even number of file pairs",
                self.inputs.len()
            );
            bail!("Number of pairs must be even");
        }
        let collection =
            fastx::Collection::from_paths(&self.inputs, fastx::CollectionType::Paired)?;
        Ok(collection)
    }
}
