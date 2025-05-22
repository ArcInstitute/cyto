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
impl MapperOffset {
    pub fn value(&self) -> usize {
        match self {
            MapperOffset::LeftOf(offset) | MapperOffset::RightOf(offset) => *offset,
        }
    }
}
impl From<MapperOffset> for usize {
    fn from(offset: MapperOffset) -> usize {
        offset.value()
    }
}

/// Describes the Remapping behavior of a `Mapper`.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum Adjustment {
    /// No positional adjustment is performed.
    #[default]
    Centered,
    /// Adjusts the position to the right.
    Plus,
    /// Adjusts the position to the left.
    Minus,
}

pub trait Mapper: Clone + Send + Sync {
    fn map_inner(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError>;

    fn map_corrected_inner(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError>;

    fn map(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        match self.map_inner(seq, offset, adjustment) {
            Ok(idx) => Ok(idx),
            Err(err) => {
                if let Some(adj) = adjustment {
                    match adj {
                        Adjustment::Centered => self.map(seq, offset, Some(Adjustment::Plus)),
                        Adjustment::Plus => self.map(seq, offset, Some(Adjustment::Minus)),
                        Adjustment::Minus => self.map(seq, offset, None),
                    }
                } else {
                    Err(err)
                }
            }
        }
    }

    fn map_corrected(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        match self.map_corrected_inner(seq, offset, adjustment) {
            Ok(idx) => Ok(idx),
            Err(err) => {
                if let Some(adj) = adjustment {
                    match adj {
                        Adjustment::Centered => self.map(seq, offset, Some(Adjustment::Plus)),
                        Adjustment::Plus => self.map(seq, offset, Some(Adjustment::Minus)),
                        Adjustment::Minus => self.map(seq, offset, None),
                    }
                } else {
                    Err(err)
                }
            }
        }
    }

    fn library_statistics(&self) -> Library;
}

impl<M: Mapper + Send + Sync> Mapper for &M {
    fn map_inner(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        (*self).map_inner(seq, offset, adjustment)
    }

    fn map_corrected_inner(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        (*self).map_corrected_inner(seq, offset, adjustment)
    }

    fn library_statistics(&self) -> Library {
        (*self).library_statistics()
    }
}
