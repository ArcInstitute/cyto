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

#[derive(Parser)]
#[clap(next_help_heading = "Binseq input options")]
pub struct BinseqInput {
    #[clap(short = 'b', long, conflicts_with_all = ["r1", "r2"])]
    pub input: Option<String>,

    /// number of threads to use for decoding and processing records.
    ///
    /// if 0, the number of threads will be set to the maximum number of threads.
    /// otherwise it will be the minimum of the provided threads and the maximum number of threads.
    #[clap(short = 't', long, default_value = "0")]
    pub threads: usize,
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

    pub fn num_threads(&self) -> usize {
        if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads.min(num_cpus::get())
        }
    }
}
