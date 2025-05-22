use clap::Parser;

#[derive(Debug, Clone, Copy, Parser)]
#[clap(next_help_heading = "Mapping Options")]
pub struct MapOptions {
    /// Use exact matching for sequences and/or probes.
    ///
    /// Default allows for unambiguous 1-hamming distance mismatches
    #[clap(short = 'x', long)]
    pub exact_matching: bool,

    /// Remap sequences and/or probes with positional adjustment
    #[clap(short = 'a', long)]
    pub adjustment: bool,
}
