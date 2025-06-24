use std::sync::Arc;

use anyhow::Result;

use super::{
    mapper::Adjustment,
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
    index_to_unit: MapIndexToName,
    index_to_aggr: MapIndexToName,
}
impl FlexMapper {
    pub fn from_tsv(filepath: &str) -> Result<Self> {
        let lib = FlexLibrary::from_tsv(filepath.into())?;
        lib.into_mapper()
    }

    pub fn from_tsv_arc(filepath: &str) -> Result<Arc<Self>> {
        let mapper = Self::from_tsv(filepath)?;
        Ok(Arc::new(mapper))
    }

    pub fn new(library: FlexLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_unit = MapIndexToName::with_capacity(library.len());
        let mut index_to_aggr = MapIndexToName::with_capacity(library.len());

        library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, flex)| -> Result<()> {
                sequence_to_index.insert(&flex.sequence, index)?;
                index_to_unit.insert(index, flex.unit_name);
                index_to_aggr.insert(index, flex.aggr_name);
                Ok(())
            })?;

        Ok(Self {
            sequence_to_index,
            index_to_unit,
            index_to_aggr,
        })
    }

    /// Retrieve the sub name of the index
    pub fn get_name(&self, index: usize) -> Option<&Name> {
        self.index_to_unit.get(index)
    }

    pub fn get_sequence_size(&self) -> usize {
        self.sequence_to_index.sequence_size
    }
}

impl Mapper for FlexMapper {
    fn map_inner(
        &self,
        seq: SeqRef,
        _offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        let flex_sequence = match adjustment {
            Some(Adjustment::Plus) => &seq[1..=self.sequence_to_index.sequence_size],
            Some(Adjustment::Minus) => return Err(MappingError::MissingFlexSequence), // Cannot map minus adjustment
            _ => &seq[..self.sequence_to_index.sequence_size],
        };
        if let Some(index) = self.sequence_to_index.match_sequence(flex_sequence) {
            Ok(index)
        } else {
            Err(MappingError::MissingFlexSequence)
        }
    }

    fn map_corrected_inner(
        &self,
        seq: SeqRef,
        _offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        let flex_sequence = match adjustment {
            Some(Adjustment::Plus) => &seq[1..=self.sequence_to_index.sequence_size],
            Some(Adjustment::Minus) => return Err(MappingError::MissingFlexSequence), // Cannot map minus adjustment
            _ => &seq[..self.sequence_to_index.sequence_size],
        };
        if let Some(index) = self
            .sequence_to_index
            .match_corrected_sequence(flex_sequence)
        {
            Ok(index)
        } else {
            Err(MappingError::MissingFlexSequence)
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
    type Record = (&'a str, &'a str);
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record> {
        assert_eq!(
            self.index_to_unit.len(),
            self.index_to_aggr.len(),
            "Error in expected index to * size"
        );
        self.index_to_unit
            .iter_records()
            .zip(self.index_to_aggr.iter_records())
    }
}
