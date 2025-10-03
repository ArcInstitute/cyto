use clap::Parser;

#[derive(Debug, Clone, Copy, Parser)]
#[clap(next_help_heading = "Mapping Options")]
pub struct MapOptions {
    /// Use exact matching for sequences and/or probes.
    ///
    /// Default allows for unambiguous 1-hamming distance mismatches
    #[clap(short = 'x', long)]
    pub exact_matching: bool,

    /// Skip quality check for UMI sequences
    ///
    /// Default removes UMIs if a quality score is below a fixed threshold
    #[clap(long)]
    pub no_umi_quality_check: bool,

    /// Never remap sequences and/or probes with +-1 position adjustment
    #[clap(long)]
    no_remap: bool,
}
impl MapOptions {
    pub fn adjustment(&self) -> bool {
        !self.no_remap
    }
    pub fn umi_quality_removal(&self) -> bool {
        !self.no_umi_quality_check
    }
}
