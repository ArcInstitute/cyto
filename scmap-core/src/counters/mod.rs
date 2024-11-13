mod barcode_index;
mod bus;
mod probe;
mod tracked_index;

pub use barcode_index::BarcodeIndexCounter;
pub use bus::BusCounter;
use hashbrown::HashMap;
pub use probe::{ProbeBarcodeIndexCounter, ProbeBusCounter};
pub use tracked_index::TrackedIndexCounter;

type Barcode = Vec<u8>;
type Umi = Vec<u8>;
type Index = usize;

type BarcodeSet = HashMap<Barcode, UmiSet>;
type UmiSet = HashMap<Umi, TrackedIndexCounter>;
type IndexCounts = HashMap<Index, usize>;
