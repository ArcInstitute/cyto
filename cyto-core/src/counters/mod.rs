mod barcode_index;
mod bus;
mod counter;
mod probe;
mod tracked_index;

pub use barcode_index::BarcodeIndexCounter;
pub use bus::BusCounter;
pub use counter::Counter;
use hashbrown::HashMap;
pub use probe::{ProbeBarcodeIndexCounter, ProbeBusCounter};
pub use tracked_index::TrackedIndexCounter;

type Barcode = u64;
type Umi = u64;
type Index = usize;

type BarcodeSet = HashMap<Barcode, UmiSet>;
type UmiSet = HashMap<Umi, TrackedIndexCounter>;
type IndexCounts = HashMap<Index, usize>;
