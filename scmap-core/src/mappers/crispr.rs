use super::{
    mapper::MapperOffset,
    maps::crispr::{MapAnchorToSequence, MapIndexToName, MapSequenceToIndex},
    Mapper,
};
use crate::{aliases::Name, libraries::CrisprLibrary};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CrisprMapper {
    anchor_to_sequence: MapAnchorToSequence,
    index_to_name: MapIndexToName,
}
impl CrisprMapper {
    pub fn new(guide_library: CrisprLibrary) -> Result<Self> {
        let mut anchor_to_sequence = MapAnchorToSequence::default();
        let mut index_to_name = MapIndexToName::default();

        guide_library
            .into_iter()
            .enumerate()
            .map(|(index, guide)| {
                anchor_to_sequence.insert(guide.anchor, guide.sequence, index)?;
                index_to_name.insert(index, guide.name);
                Ok(())
            })
            .collect::<Result<()>>()?;

        Ok(Self {
            anchor_to_sequence,
            index_to_name,
        })
    }
    /// Maps an input sequence to a potential set of guides through an anchor sequence.
    fn map_anchor(&self, sequence: &[u8], offset: usize) -> Option<(usize, &MapSequenceToIndex)> {
        for anchor_size in self.anchor_to_sequence.anchor_sizes.iter() {
            let anchor = &sequence[offset..offset + anchor_size];
            if let Some(sequence_map) = self.anchor_to_sequence.get_sequence_map(anchor) {
                return Some((*anchor_size, sequence_map));
            }
        }
        None
    }

    /// Maps an input sequence to a guide name through a sequence.
    fn map_sequence(
        &self,
        sequence: &[u8],
        sequence_map: &MapSequenceToIndex,
        offset: usize,
        anchor_size: usize,
    ) -> Option<usize> {
        let lpos = offset + anchor_size;
        let rpos = lpos + self.anchor_to_sequence.sequence_size;
        let sequence = &sequence[lpos..rpos];
        sequence_map.get(sequence).copied()
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
    fn map(&self, sequence: &[u8], offset: Option<MapperOffset>) -> Option<usize> {
        assert!(offset.is_some(), "CrisprMapper requires an offset");
        let offset = offset.unwrap();
        let (anchor_size, sequence_map) = self.map_anchor(sequence, offset.into())?;
        self.map_sequence(sequence, sequence_map, offset.into(), anchor_size)
    }
}
