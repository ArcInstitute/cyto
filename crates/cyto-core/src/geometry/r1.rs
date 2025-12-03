use ibu::Header;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeometryR1 {
    pub barcode: usize,
    pub umi: usize,
}
impl TryFrom<GeometryR1> for Header {
    type Error = anyhow::Error;

    #[allow(clippy::cast_possible_truncation)]
    fn try_from(geometry: GeometryR1) -> Result<Self, Self::Error> {
        let header = Header::new(
            geometry.barcode as u32, // Barcode size
            geometry.umi as u32,     // UMI size
        );
        header.validate()?;
        Ok(header)
    }
}
