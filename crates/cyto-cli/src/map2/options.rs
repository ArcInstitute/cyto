use clap::Parser;

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Mapping Options")]
pub struct Map2Options {
    /// Geometry DSL string
    ///
    /// default-gex: [barcode][umi:12]|[gex][:18][probe]
    ///
    /// default-crispr: [barcode][umi:12]|[:18][probe][anchor][protospacer]
    #[clap(short = 'g', long)]
    pub geometry: Option<String>,

    /// Path to whitelist file
    #[clap(short = 'w', long)]
    pub whitelist: Option<String>,

    /// Path to probe file
    #[clap(short = 'p', long)]
    pub probes: Option<String>,

    /// Use exact matching (no hamming distance correction)
    #[clap(short = 'x', long)]
    pub exact: bool,

    /// Remap window size for position adjustment (0 to disable)
    #[clap(long, default_value = "1")]
    pub remap_window: usize,

    /// Skip UMI quality check
    #[clap(long)]
    pub no_umi_qual_check: bool,
}
