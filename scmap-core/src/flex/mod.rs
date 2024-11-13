mod flex;
mod library;
mod mapper;
mod maps;

pub use flex::Flex;
pub use library::Library;
pub use mapper::Mapper;
pub use maps::{MapIndexToName, MapSequenceToIndex};

pub type Sequence = Vec<u8>;
pub type Name = Vec<u8>;
