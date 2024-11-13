use super::{Library, MapIndexToName, MapSequenceToIndex, Name};
use anyhow::Result;

#[derive(Debug)]
pub struct Mapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_name: MapIndexToName,
}
impl Mapper {
    pub fn new(flex_library: Library) -> Result<Self> {
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

    pub fn map(&self, sequence: &[u8]) -> Option<usize> {
        let flex_sequence = &sequence[..self.sequence_to_index.sequence_size];
        self.sequence_to_index.get(flex_sequence)
    }

    pub fn get_name(&self, index: usize) -> Option<&Name> {
        self.index_to_name.get(index)
    }

    pub fn get_sequence_size(&self) -> usize {
        self.sequence_to_index.sequence_size
    }
}
