mod library;
mod mapper;
mod maps;
mod probe;

pub use library::Library;
pub use mapper::Mapper;
pub use maps::{MapIndexToAlias, MapSequenceToIndex};
pub use probe::{Probe, ProbeAlias};

pub type Sequence = Vec<u8>;
pub type AliasNuc = Vec<u8>;
pub type Alias = Vec<u8>;
