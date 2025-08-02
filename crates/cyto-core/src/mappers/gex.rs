use std::{path::Path, sync::Arc};

use anyhow::Result;

use super::{
    mapper::Adjustment,
    maps::gex::{MapIndexToName, MapSequenceToIndex},
    Mapper, MapperOffset, MappingError,
};
use crate::{
    aliases::{Name, SeqRef},
    io::FeatureWriter,
    libraries::GexLibrary,
    statistics::{GexLibraryStatistics, Library},
};

#[derive(Debug, Clone)]
pub struct GexMapper {
    sequence_to_index: MapSequenceToIndex,
    index_to_unit: MapIndexToName,
    index_to_aggr: MapIndexToName,
}
impl GexMapper {
    pub fn from_tsv<P: AsRef<Path>>(filepath: P) -> Result<Self> {
        let lib = GexLibrary::from_tsv(filepath)?;
        lib.into_mapper()
    }

    pub fn from_tsv_arc<P: AsRef<Path>>(filepath: P) -> Result<Arc<Self>> {
        let mapper = Self::from_tsv(filepath)?;
        Ok(Arc::new(mapper))
    }

    pub fn new(library: GexLibrary) -> Result<Self> {
        let mut sequence_to_index = MapSequenceToIndex::default();
        let mut index_to_unit = MapIndexToName::with_capacity(library.len());
        let mut index_to_aggr = MapIndexToName::with_capacity(library.len());

        library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, gex)| -> Result<()> {
                sequence_to_index.insert(&gex.sequence, index)?;
                index_to_unit.insert(index, gex.unit_name);
                index_to_aggr.insert(index, gex.aggr_name);
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

impl Mapper for GexMapper {
    fn map_inner(
        &self,
        seq: SeqRef,
        _offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        let gex_sequence = match adjustment {
            Some(Adjustment::Plus) => &seq[1..=self.sequence_to_index.sequence_size.max(seq.len())],
            Some(Adjustment::Minus) => return Err(MappingError::MissingGexSequence), // Cannot map minus adjustment
            _ => &seq[..self.sequence_to_index.sequence_size.max(seq.len())],
        };
        if seq.len() < self.sequence_to_index.sequence_size {
            return Err(MappingError::UnexpectedlyTruncated);
        }
        if let Some(index) = self.sequence_to_index.match_sequence(gex_sequence) {
            Ok(index)
        } else {
            Err(MappingError::MissingGexSequence)
        }
    }

    fn map_corrected_inner(
        &self,
        seq: SeqRef,
        _offset: Option<MapperOffset>,
        adjustment: Option<Adjustment>,
    ) -> Result<usize, MappingError> {
        let gex_sequence = match adjustment {
            Some(Adjustment::Plus) => &seq[1..=self.sequence_to_index.sequence_size.max(seq.len())],
            Some(Adjustment::Minus) => return Err(MappingError::MissingGexSequence), // Cannot map minus adjustment
            _ => &seq[..self.sequence_to_index.sequence_size.max(seq.len())],
        };
        if gex_sequence.len() < self.sequence_to_index.sequence_size {
            return Err(MappingError::UnexpectedlyTruncated);
        }

        if let Some(index) = self
            .sequence_to_index
            .match_corrected_sequence(gex_sequence)
        {
            Ok(index)
        } else {
            Err(MappingError::MissingGexSequence)
        }
    }

    fn library_statistics(&self) -> Library {
        let statistics = GexLibraryStatistics {
            num_gex_sequences: self.sequence_to_index.len(),
            gex_sequence_size: self.sequence_to_index.sequence_size,
        };
        Library::Gex(statistics)
    }
}

impl<'a> FeatureWriter<'a> for GexMapper {
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
