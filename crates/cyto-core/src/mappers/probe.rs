use std::{path::Path, sync::Arc};

use anyhow::Result;
use disambiseq::Disambibyte;
use log::info;

use super::{
    mapper::Adjustment,
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
    pub sequence_to_index: MapSequenceToIndex,
    pub index_to_alias: MapIndexToAlias,
    corrected: Disambibyte,
}
impl ProbeMapper {
    pub fn from_tsv<P: AsRef<Path>>(filepath: P, exact_match: bool) -> Result<Self> {
        let lib = ProbeLibrary::from_tsv(filepath)?;
        if exact_match {
            lib.into_mapper()
        } else {
            lib.into_corrected_mapper()
        }
    }

    pub fn from_tsv_arc<P: AsRef<Path>>(filepath: P, exact_match: bool) -> Result<Arc<Self>> {
        let mapper = Self::from_tsv(filepath, exact_match)?;
        Ok(Arc::new(mapper))
    }

    pub fn new(probe_library: ProbeLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_alias = MapIndexToAlias::default();

        info!("Building exact Flex multiplexing probe mapper");
        probe_library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, probe)| -> Result<()> {
                sequence_to_index.insert(probe.sequence, index)?;
                index_to_alias.insert(index, probe.alias_nuc, probe.alias);
                Ok(())
            })?;

        Ok(Self {
            sequence_to_index,
            index_to_alias,
            corrected: Disambibyte::default(),
        })
    }

    pub fn new_corrected(probe_library: ProbeLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_alias = MapIndexToAlias::default();
        let mut corrected = Disambibyte::default();

        info!("Building disambiguated one-off Flex multiplexing probe mapper");
        probe_library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, probe)| -> Result<()> {
                corrected.insert(&probe.sequence);
                sequence_to_index.insert(probe.sequence, index)?;
                index_to_alias.insert(index, probe.alias_nuc, probe.alias);
                Ok(())
            })?;
        info!("Finished disambiguation");

        Ok(Self {
            sequence_to_index,
            index_to_alias,
            corrected,
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

    /// Maps the sequence to the left of the offset to an index.
    ///
    /// First corrects the sequence to a sequence in the library if possible.
    fn map_left_corrected(&self, sequence: SeqRef, offset: usize) -> Result<usize, MappingError> {
        let rpos = offset;
        let lpos = rpos - self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        if let Some(seq) = self.corrected.get_parent(subsequence) {
            if let Some(index) = self.sequence_to_index.get(&seq.0) {
                Ok(index)
            } else {
                Err(MappingError::MissingProbe)
            }
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

    /// Maps the sequence to the right of the offset to an index.
    ///
    /// First corrects the sequence to a sequence in the library if possible.
    fn map_right_corrected(&self, sequence: SeqRef, offset: usize) -> Result<usize, MappingError> {
        let lpos = offset;
        let rpos = lpos + self.sequence_to_index.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        if let Some(seq) = self.corrected.get_parent(subsequence) {
            if let Some(index) = self.sequence_to_index.get(&seq.0) {
                Ok(index)
            } else {
                Err(MappingError::MissingProbe)
            }
        } else {
            Err(MappingError::MissingProbe)
        }
    }

    pub fn get_alias(&self, index: usize) -> Option<&ProbeAlias> {
        self.index_to_alias.get(index)
    }

    pub fn get_alias_index(&self, index: usize) -> Option<usize> {
        self.index_to_alias.get_index(index)
    }

    pub fn num_unique_aliases(&self) -> usize {
        self.index_to_alias.num_unique_aliases()
    }
}

impl Mapper for ProbeMapper {
    fn map_inner(
        &self,
        sequence: SeqRef,
        offset: Option<MapperOffset>,
        _adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        match offset {
            Some(MapperOffset::LeftOf(offset)) => self.map_left(sequence, offset),
            Some(MapperOffset::RightOf(offset)) => self.map_right(sequence, offset),
            None => panic!("ProbeMapper requires an offset to map the sequence."),
        }
    }
    fn map_corrected_inner(
        &self,
        sequence: SeqRef,
        offset: Option<MapperOffset>,
        _adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        match offset {
            Some(MapperOffset::LeftOf(offset)) => self.map_left_corrected(sequence, offset),
            Some(MapperOffset::RightOf(offset)) => self.map_right_corrected(sequence, offset),
            None => panic!("ProbeMapper requires an offset to map the sequence."),
        }
    }
    fn library_statistics(&self) -> Library {
        let statistics = ProbeLibraryStatistics {
            num_probes: self.sequence_to_index.len(),
            num_aliases: self.index_to_alias.num_unique_aliases(),
            probe_size: self.sequence_to_index.sequence_size,
        };
        Library::Probe(statistics)
    }
}
