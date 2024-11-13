use super::{
    maps::flex::{MapIndexToName, MapSequenceToIndex},
    Mapper, MapperOffset,
};
use crate::{aliases::Name, libraries::FlexLibrary};
use anyhow::Result;

#[derive(Debug)]
pub struct FlexMapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_name: MapIndexToName,
}
impl FlexMapper {
    pub fn new(flex_library: FlexLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_name = MapIndexToName::default();

        flex_library
            .into_iter()
            .enumerate()
            .map(|(index, flex)| {
                sequence_to_index.insert(flex.sequence, index)?;
                index_to_name.insert(index, flex.name);
                Ok(())
            })
            .collect::<Result<()>>()?;

        Ok(Self {
            sequence_to_index,
            index_to_name,
        })
    }

    pub fn get_name(&self, index: usize) -> Option<&Name> {
        self.index_to_name.get(index)
    }

    pub fn get_sequence_size(&self) -> usize {
        self.sequence_to_index.sequence_size
    }
}

impl Mapper for FlexMapper {
    fn map(&self, seq: &[u8], _offset: Option<MapperOffset>) -> Option<usize> {
        let flex_sequence = &seq[..self.sequence_to_index.sequence_size];
        self.sequence_to_index.get(flex_sequence)
    }
}
