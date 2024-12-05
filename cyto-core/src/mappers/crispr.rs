use anyhow::Result;

use super::{
    mapper::MapperOffset,
    maps::crispr::{MapAnchorToSequence, MapIndexToName, MapSequenceToIndex},
    Mapper, MappingError,
};
use crate::{
    aliases::{Name, SeqRef},
    io::FeatureWriter,
    libraries::CrisprLibrary,
    statistics::{CrisprLibraryStatistics, Library},
};

#[derive(Debug, Clone)]
pub struct CrisprMapper {
    anchor_to_sequence: MapAnchorToSequence,
    index_to_name: MapIndexToName,
}
impl CrisprMapper {
    pub fn new(guide_library: CrisprLibrary) -> Result<Self> {
        let mut anchor_to_sequence = MapAnchorToSequence::default();
        let mut index_to_name = MapIndexToName::with_capacity(guide_library.len());

        guide_library
            .into_iter()
            .enumerate()
            .try_for_each(|(index, guide)| -> Result<()> {
                anchor_to_sequence.insert(guide.anchor, guide.sequence, index)?;
                index_to_name.insert(index, guide.name);
                Ok(())
            })?;

        Ok(Self {
            anchor_to_sequence,
            index_to_name,
        })
    }
    /// Maps an input sequence to a potential set of guides through an anchor sequence.
    fn map_anchor(
        &self,
        sequence: SeqRef,
        offset: usize,
    ) -> Result<(usize, &MapSequenceToIndex), MappingError> {
        for anchor_size in &self.anchor_to_sequence.anchor_sizes {
            let anchor = &sequence[offset..offset + anchor_size];
            if let Some(sequence_map) = self.anchor_to_sequence.get_sequence_map(anchor) {
                return Ok((*anchor_size, sequence_map));
            }
        }
        Err(MappingError::MissingAnchor)
    }

    /// Maps an input sequence to a guide name through a sequence.
    fn map_sequence(
        &self,
        sequence: SeqRef,
        sequence_map: &MapSequenceToIndex,
        offset: usize,
        anchor_size: usize,
    ) -> Result<usize, MappingError> {
        let lpos = offset + anchor_size;
        let rpos = lpos + self.anchor_to_sequence.sequence_size;
        let sequence = &sequence[lpos..rpos];
        if let Some(index) = sequence_map.get(sequence) {
            Ok(*index)
        } else {
            Err(MappingError::MissingProtospacer)
        }
    }

    /// Retrieves the guide name from the guide index.
    pub fn get_name(&self, index: usize) -> Option<&Name> {
        self.index_to_name.get(index)
    }

    /// Convenience method for listing all anchors (useful for debugging).
    pub fn list_anchors(&self) -> Result<()> {
        self.anchor_to_sequence.list_anchors()
    }
}

impl Mapper for CrisprMapper {
    /// Maps an input sequence to a guide name.
    ///
    /// 1. Hash the anchor sequence at the precomputed offset.
    ///     a. If found continue
    ///     b. If not found, return None
    /// 2. Hash the sequence to the expected guides for the anchor.
    ///     a. If found continue
    ///     b. If not found, return None
    /// 3. Map the guide index to the guide name.
    /// 4. Return the guide name.
    fn map(&self, sequence: SeqRef, offset: Option<MapperOffset>) -> Result<usize, MappingError> {
        assert!(offset.is_some(), "CrisprMapper requires an offset");
        let offset = offset.unwrap();
        let (anchor_size, sequence_map) = self.map_anchor(sequence, offset.into())?;
        self.map_sequence(sequence, sequence_map, offset.into(), anchor_size)
    }

    fn library_statistics(&self) -> Library {
        let statistics = CrisprLibraryStatistics {
            num_anchors: self.anchor_to_sequence.len(),
            anchor_sizes: self.anchor_to_sequence.export_anchor_sizes(),
            num_protospacers: self.index_to_name.len(),
            protospacer_size: self.anchor_to_sequence.sequence_size,
        };
        Library::Crispr(statistics)
    }
}

impl<'a> FeatureWriter<'a> for CrisprMapper {
    type Record = &'a str;
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record> {
        self.index_to_name.iter_records()
    }
}
