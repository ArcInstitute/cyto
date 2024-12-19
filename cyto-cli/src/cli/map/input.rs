use anyhow::Result;
use clap::Parser;
use cyto::PairedReader;

#[derive(Parser)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(short = 'i', long, num_args = 1..)]
    pub r1: Vec<String>,
    #[clap(short = 'I', long, num_args = 1..)]
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
        if self.r1.len() == 0 {
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
