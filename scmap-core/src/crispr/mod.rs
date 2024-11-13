mod guide;
mod library;
mod mapper;
mod maps;

pub use guide::Guide;
pub use library::Library;
pub use mapper::Mapper;
pub use maps::{MapAnchorToSequence, MapIndexToName, MapSequenceToIndex};

pub type Name = Vec<u8>;
pub type Anchor = Vec<u8>;
pub type Sequence = Vec<u8>;
