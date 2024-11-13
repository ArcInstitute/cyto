use clap::Parser;

#[derive(Parser)]
#[clap(next_help_heading = "Geometry Configuration")]
pub struct Geometry {
    #[clap(short = 'b', long, default_value = "16")]
    pub barcode: usize,
    #[clap(short = 'u', long, default_value = "12")]
    pub umi: usize,
}
