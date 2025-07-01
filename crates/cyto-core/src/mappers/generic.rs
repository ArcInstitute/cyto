use std::sync::Arc;

use anyhow::Result;
use disambiseq::Disambibyte;

use super::{
    mapper::Adjustment,
    maps::generic::{MapIndexToName, MapSequenceToIndex},
    Mapper, MapperOffset, MappingError,
};
use crate::{
    aliases::{Name, SeqRef},
    io::FeatureWriter,
    libraries::GenericLibrary,
    statistics::{GenericLibraryStatistics, Library},
};

#[derive(Debug, Clone)]
pub struct GenericMapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_unit: MapIndexToName,
    index_to_aggr: MapIndexToName,
    correction: Disambibyte,
}
impl GenericMapper {
    pub fn from_tsv(filepath: &str, exact_match: bool) -> Result<Self> {
        let library = GenericLibrary::from_tsv(filepath.into())?;
        if exact_match {
            library.into_mapper()
        } else {
            library.into_corrected_mapper()
        }
    }

    pub fn from_tsv_arc(filepath: &str, exact_match: bool) -> Result<Arc<Self>> {
        let mapper = Self::from_tsv(filepath, exact_match)?;
        Ok(Arc::new(mapper))
    }

    pub fn new(library: GenericLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_unit = MapIndexToName::with_capacity(library.len());
        let mut index_to_aggr = MapIndexToName::with_capacity(library.len());
        let correction = Disambibyte::default();

        library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, target)| -> Result<()> {
                sequence_to_index.insert(target.sequence, index)?;
                index_to_unit.insert(index, target.unit_name);
                index_to_aggr.insert(index, target.aggr_name);
                Ok(())
            })?;

        Ok(Self {
            sequence_to_index,
            index_to_unit,
            index_to_aggr,
            correction,
        })
    }

    pub fn new_corrected(library: GenericLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_unit = MapIndexToName::with_capacity(library.len());
        let mut index_to_aggr = MapIndexToName::with_capacity(library.len());
        let mut correction = Disambibyte::default();

        library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, target)| -> Result<()> {
                correction.insert(&target.sequence);
                sequence_to_index.insert(target.sequence, index)?;
                index_to_unit.insert(index, target.unit_name);
                index_to_aggr.insert(index, target.aggr_name);
                Ok(())
            })?;

        Ok(Self {
            sequence_to_index,
            index_to_unit,
            index_to_aggr,
            correction,
        })
    }

    pub fn get_name(&self, index: usize) -> Option<&Name> {
        self.index_to_unit.get(index)
    }

    pub fn get_sequence_size(&self) -> usize {
        self.sequence_to_index.sequence_size
    }

    pub fn isolate_sequence<'a>(
        &self,
        seq: &'a SeqRef,
        offset: MapperOffset,
        adjustment: Option<Adjustment>,
    ) -> Option<Result<&'a [u8], MappingError>> {
        let seq_size = self.sequence_to_index.sequence_size;
        match offset {
            MapperOffset::LeftOf(x) => {
                let x = match adjustment {
                    Some(Adjustment::Plus) => {
                        if x == seq.len() {
                            return None;
                        }
                        x + 1
                    }
                    Some(Adjustment::Minus) => {
                        if x == 0 {
                            return None;
                        }
                        x - 1
                    }
                    _ => x,
                };
                if x < seq_size || x > seq.len() {
                    return None;
                }
                Some(Ok(&seq[x - seq_size..x]))
            }
            MapperOffset::RightOf(x) => {
                let x = match adjustment {
                    Some(Adjustment::Plus) => {
                        if x == seq.len() {
                            return None;
                        }
                        x + 1
                    }
                    Some(Adjustment::Minus) => {
                        if x == 0 {
                            return None;
                        }
                        x - 1
                    }
                    _ => x,
                };
                if x + seq_size > seq.len() {
                    return Some(Err(MappingError::UnexpectedlyTruncated));
                }
                Some(Ok(&seq[x..x + seq_size]))
            }
        }
    }
}

impl Mapper for GenericMapper {
    fn map_inner(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        assert!(offset.is_some(), "GenericMapper requires an offset");
        let offset = offset.unwrap();
        let target = match self.isolate_sequence(&seq, offset, adjustment) {
            Some(target) => target?,
            None => return Err(MappingError::MissingTargetSequence),
        };
        if let Some(index) = self.sequence_to_index.get(target) {
            Ok(index)
        } else {
            Err(MappingError::MissingTargetSequence)
        }
    }

    fn map_corrected_inner(
        &self,
        seq: SeqRef,
        offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        assert!(offset.is_some(), "GenericMapper requires an offset");
        let offset = offset.unwrap();
        let target = match self.isolate_sequence(&seq, offset, adjustment) {
            Some(target) => target?,
            None => return Err(MappingError::MissingTargetSequence),
        };
        match self.correction.get_parent(target) {
            Some(corrected) => {
                if let Some(index) = self.sequence_to_index.get(&corrected.0) {
                    Ok(index)
                } else {
                    Err(MappingError::MissingTargetSequence)
                }
            }
            None => Err(MappingError::MissingTargetSequence),
        }
    }

    fn library_statistics(&self) -> Library {
        let statistics = GenericLibraryStatistics {
            num_target_sequences: self.sequence_to_index.len(),
            target_sequence_size: self.sequence_to_index.sequence_size,
        };
        Library::Generic(statistics)
    }
}

impl<'a> FeatureWriter<'a> for GenericMapper {
    type Record = (&'a str, &'a str);
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record> {
        self.index_to_unit
            .iter_records()
            .zip(self.index_to_aggr.iter_records())
    }
}
