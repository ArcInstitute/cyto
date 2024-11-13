use super::{
    maps::probe::{MapIndexToAlias, MapSequenceToIndex},
    Mapper, MapperOffset,
};
use crate::{libraries::ProbeLibrary, metadata::ProbeAlias};
use anyhow::Result;

#[derive(Debug)]
pub struct ProbeMapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_alias: MapIndexToAlias,
}
impl ProbeMapper {
    pub fn new(probe_library: ProbeLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_alias = MapIndexToAlias::default();
        probe_library
            .into_iter()
            .enumerate()
            .map(|(index, probe)| {
                sequence_to_index.insert(probe.sequence, index)?;
                index_to_alias.insert(index, probe.alias_nuc, probe.alias);
                Ok(())
            })
            .collect::<Result<()>>()?;
        Ok(Self {
            sequence_to_index,
            index_to_alias,
        })
    }

    /// Maps the sequence to the left of the offset to an index.
    fn map_left(&self, sequence: &[u8], offset: usize) -> Option<usize> {
        let rpos = offset;
        let lpos = rpos - self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        self.sequence_to_index.get(subsequence)
    }

    /// Maps the sequence to the right of the offset to an index.
    fn map_right(&self, sequence: &[u8], offset: usize) -> Option<usize> {
        let lpos = offset;
        let rpos = lpos + self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        self.sequence_to_index.get(subsequence)
    }

    pub fn get_alias(&self, index: usize) -> Option<&ProbeAlias> {
        self.index_to_alias.get(index)
    }
}

impl Mapper for ProbeMapper {
    fn map(&self, sequence: &[u8], offset: Option<MapperOffset>) -> Option<usize> {
        match offset {
            Some(MapperOffset::LeftOf(offset)) => self.map_left(sequence, offset),
            Some(MapperOffset::RightOf(offset)) => self.map_right(sequence, offset),
            None => panic!("ProbeMapper requires an offset to map the sequence."),
        }
    }
}
