use super::MappingError;
use crate::{aliases::SeqRef, statistics::Library};

/// Describes the offset to provide to the underlying `Mapper` implementation.
///
/// Each `Mapper` implementation should be able to handle the offset in a way that makes sense for
/// the underlying data structure.
#[derive(Debug, Clone, Copy)]
pub enum MapperOffset {
    RightOf(usize),
    LeftOf(usize),
}
impl From<MapperOffset> for usize {
    fn from(offset: MapperOffset) -> usize {
        match offset {
            MapperOffset::LeftOf(offset) | MapperOffset::RightOf(offset) => offset,
        }
    }
}

pub trait Mapper: Clone + Send {
    fn map(&self, seq: SeqRef, offset: Option<MapperOffset>) -> Result<usize, MappingError>;

    fn library_statistics(&self) -> Library;
}

impl<M: Mapper + Sync> Mapper for &M {
    fn map(&self, seq: SeqRef, offset: Option<MapperOffset>) -> Result<usize, MappingError> {
        (*self).map(seq, offset)
    }
    fn library_statistics(&self) -> Library {
        (*self).library_statistics()
    }
}
