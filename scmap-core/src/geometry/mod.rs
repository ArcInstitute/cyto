mod bus;
pub use bus::Bus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeometryR1 {
    pub barcode: usize,
    pub umi: usize,
}
