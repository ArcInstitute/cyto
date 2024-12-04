use clap::Parser;
use scmap::GeometryR1;

#[derive(Parser)]
#[clap(next_help_heading = "Geometry Configuration")]
pub struct Geometry {
    #[clap(short = 'b', long, default_value = "16")]
    pub barcode: usize,
    #[clap(short = 'u', long, default_value = "12")]
    pub umi: usize,
}

impl From<Geometry> for GeometryR1 {
    fn from(geometry: Geometry) -> Self {
        Self {
            barcode: geometry.barcode,
            umi: geometry.umi,
        }
    }
}
