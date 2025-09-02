use clap::Parser;

#[derive(Debug, Clone, Copy, Parser)]
#[clap(next_help_heading = "Mapping Options")]
pub struct MapOptions {
    /// Use exact matching for sequences and/or probes.
    ///
    /// Default allows for unambiguous 1-hamming distance mismatches
    #[clap(short = 'x', long)]
    pub exact_matching: bool,

    /// Never remap sequences and/or probes with +-1 position adjustment
    #[clap(long)]
    pub no_remap: bool,
}
impl MapOptions {
    pub fn adjustment(&self) -> bool {
        !self.no_remap
    }
}
