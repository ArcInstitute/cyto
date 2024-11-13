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
            MapperOffset::LeftOf(offset) => offset,
            MapperOffset::RightOf(offset) => offset,
        }
    }
}

pub trait Mapper {
    fn map(&self, seq: &[u8], offset: Option<MapperOffset>) -> Option<usize>;
}

impl<M: Mapper> Mapper for &M {
    fn map(&self, seq: &[u8], offset: Option<MapperOffset>) -> Option<usize> {
        (*self).map(seq, offset)
    }
}
