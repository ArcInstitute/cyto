use anyhow::Result;
use clap::Parser;
use cyto::PairedReader;

#[cfg(feature = "binseq")]
pub use anyhow::bail;
#[cfg(feature = "binseq")]
pub use binseq::PairedMmapReader;

#[derive(Parser)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(short = 'i', long, num_args = 1.., required_unless_present="input")]
    pub r1: Vec<String>,
    #[clap(short = 'I', long, num_args = 1.., required_unless_present="input")]
    pub r2: Vec<String>,
}
impl PairedInput {
    pub fn valid_sizing(&self) -> bool {
        self.r1.len() == self.r2.len()
    }
    pub fn iter_pairs(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.r1
            .iter()
            .zip(self.r2.iter())
            .map(|(r1, r2)| (r1.clone(), r2.clone()))
    }
    #[allow(clippy::wrong_self_convention)]
    pub fn into_readers(&self) -> Result<Vec<PairedReader>> {
        if self.r1.is_empty() {
            anyhow::bail!("No input files provided");
        }
        if !self.valid_sizing() {
            anyhow::bail!("Number of R1 and R2 files must match");
        }
        self.iter_pairs()
            .map(|(r1, r2)| PairedReader::new(&r1, &r2))
            .collect()
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
