use anyhow::Result;
use disambiseq::Disambibyte;

use super::{
    maps::flex::{MapIndexToName, MapSequenceToIndex},
    Mapper, MapperOffset, MappingError,
};
use crate::{
    aliases::{Name, SeqRef},
    io::FeatureWriter,
    libraries::FlexLibrary,
    statistics::{FlexLibraryStatistics, Library},
};

#[derive(Debug, Clone)]
pub struct FlexMapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_name: MapIndexToName,
    correction: Disambibyte,
}
impl FlexMapper {
    pub fn new(library: FlexLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_name = MapIndexToName::with_capacity(library.len());
        let correction = Disambibyte::default();

        library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, flex)| -> Result<()> {
                sequence_to_index.insert(flex.sequence, index)?;
                index_to_name.insert(index, flex.name);
                Ok(())
            })?;

        Ok(Self {
            sequence_to_index,
            index_to_name,
            correction,
        })
    }

    pub fn new_corrected(library: FlexLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_name = MapIndexToName::with_capacity(library.len());
        let mut correction = Disambibyte::default();

        library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, flex)| -> Result<()> {
                correction.insert(&flex.sequence);
                sequence_to_index.insert(flex.sequence, index)?;
                index_to_name.insert(index, flex.name);
                Ok(())
            })?;

        Ok(Self {
            sequence_to_index,
            index_to_name,
            correction,
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
    fn map(&self, seq: SeqRef, _offset: Option<MapperOffset>) -> Result<usize, MappingError> {
        let flex_sequence = &seq[..self.sequence_to_index.sequence_size];
        if let Some(index) = self.sequence_to_index.get(flex_sequence) {
            Ok(index)
        } else {
            Err(MappingError::MissingFlexSequence)
        }
    }

    fn map_corrected(
        &self,
        seq: SeqRef,
        _offset: Option<MapperOffset>,
    ) -> Result<usize, MappingError> {
        let flex_sequence = &seq[..self.sequence_to_index.sequence_size];
        match self.correction.get_parent(flex_sequence) {
            Some(corrected) => {
                if let Some(index) = self.sequence_to_index.get(&corrected.0) {
                    Ok(index)
                } else {
                    Err(MappingError::MissingFlexSequence)
                }
            }
            None => Err(MappingError::MissingFlexSequence),
        }
    }

    fn library_statistics(&self) -> Library {
        let statistics = FlexLibraryStatistics {
            num_flex_sequences: self.sequence_to_index.len(),
            flex_sequence_size: self.sequence_to_index.sequence_size,
        };
        Library::Flex(statistics)
    }
}

impl<'a> FeatureWriter<'a> for FlexMapper {
    type Record = &'a str;
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record> {
        Box::new(self.index_to_name.iter_records())
    }
}
