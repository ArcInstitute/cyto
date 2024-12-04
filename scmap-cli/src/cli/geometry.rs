use clap::Parser;
use ibu::Header;
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
impl TryFrom<Geometry> for Header {
    type Error = anyhow::Error;

    #[allow(clippy::cast_possible_truncation)]
    fn try_from(geometry: Geometry) -> Result<Self, Self::Error> {
        let header = Header::new(
            1,                       // IBU version
            geometry.barcode as u32, // Barcode size
            geometry.umi as u32,     // UMI size
            false,                   // Sorted flag
        )?;
        Ok(header)
    }
}
