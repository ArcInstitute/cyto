use anyhow::Result;

use super::{Library, MapIndexToAlias, MapSequenceToIndex, ProbeAlias};

#[derive(Debug)]
pub struct Mapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_alias: MapIndexToAlias,
}
impl Mapper {
    pub fn new(probe_library: Library) -> Result<Self> {
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

    pub fn map(&self, sequence: &[u8], offset: usize) -> Option<usize> {
        let rpos = offset;
        let lpos = rpos - self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        self.sequence_to_index.get(subsequence)
    }

    pub fn get_alias(&self, index: usize) -> Option<&ProbeAlias> {
        self.index_to_alias.get(index)
    }
}
