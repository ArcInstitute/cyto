use anyhow::Result;

use super::{
    maps::probe::{MapIndexToAlias, MapSequenceToIndex},
    Mapper, MapperOffset, MappingError,
};
use crate::{
    aliases::SeqRef,
    libraries::ProbeLibrary,
    metadata::ProbeAlias,
    statistics::{Library, ProbeLibraryStatistics},
};

#[derive(Debug, Clone)]
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
    fn map_left(&self, sequence: SeqRef, offset: usize) -> Result<usize, MappingError> {
        let rpos = offset;
        let lpos = rpos - self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        if let Some(index) = self.sequence_to_index.get(subsequence) {
            Ok(index)
        } else {
            Err(MappingError::MissingProbe)
        }
    }

    /// Maps the sequence to the right of the offset to an index.
    fn map_right(&self, sequence: SeqRef, offset: usize) -> Result<usize, MappingError> {
        let lpos = offset;
        let rpos = lpos + self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        if let Some(index) = self.sequence_to_index.get(subsequence) {
            Ok(index)
        } else {
            Err(MappingError::MissingProbe)
        }
    }

    pub fn get_alias(&self, index: usize) -> Option<&ProbeAlias> {
        self.index_to_alias.get(index)
    }
}

impl Mapper for ProbeMapper {
    fn map(&self, sequence: SeqRef, offset: Option<MapperOffset>) -> Result<usize, MappingError> {
        match offset {
            Some(MapperOffset::LeftOf(offset)) => self.map_left(sequence, offset),
            Some(MapperOffset::RightOf(offset)) => self.map_right(sequence, offset),
            None => panic!("ProbeMapper requires an offset to map the sequence."),
        }
    }
    fn library_statistics(&self) -> Library {
        let statistics = ProbeLibraryStatistics {
            num_probes: self.sequence_to_index.len(),
            num_aliases: self.index_to_alias.len(),
            probe_size: self.sequence_to_index.sequence_size,
        };
        Library::Probe(statistics)
    }
}
