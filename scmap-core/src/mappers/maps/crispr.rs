use crate::aliases::{Anchor, Name, Sequence};
use anyhow::{bail, Result};
use hashbrown::{HashMap, HashSet};

#[derive(Default, Debug)]
pub struct MapIndexToName {
    map: HashMap<usize, Name>,
}
impl MapIndexToName {
    /// Insert a name into the map
    pub fn insert(&mut self, index: usize, name: Name) {
        self.map.insert(index, name);
    }
    /// Get a name from the map
    pub fn get(&self, index: usize) -> Option<&Name> {
        self.map.get(&index)
    }
}

#[derive(Default, Debug)]
pub struct MapSequenceToIndex {
    map: HashMap<Sequence, usize>,
}
impl MapSequenceToIndex {
    /// Insert a sequence into the map
    pub fn insert(&mut self, sequence: Sequence, index: usize) {
        self.map.insert(sequence, index);
    }

    /// Get a sequence from the map
    pub fn get(&self, sequence: &[u8]) -> Option<&usize> {
        self.map.get(sequence)
    }
}

#[derive(Default, Debug)]
pub struct MapAnchorToSequence {
    map: HashMap<Anchor, MapSequenceToIndex>,
    pub anchor_sizes: HashSet<usize>,
    pub sequence_size: usize,
}
impl MapAnchorToSequence {
    /// Update the sequence size
    ///
    /// This function will update the sequence size if it is not already set.
    /// If the sequence size is already set, it will check if the new sequence
    /// has the same size as the existing sequence size. If the sizes do not
    /// match, an error will be returned.
    fn update_sequence_size(&mut self, index: usize, sequence: &Sequence) -> Result<()> {
        if self.sequence_size == 0 || self.sequence_size == sequence.len() {
            self.sequence_size = sequence.len();
            Ok(())
        } else {
            let sequence_str = std::str::from_utf8(&sequence)?;
            let expected_size = self.sequence_size;
            let observed_size = sequence.len();
            bail!(
                "Guide sequence size mismatch occured at guide: {index}\nExpected size: {expected_size}\nFound size: {observed_size}\nSequence: {sequence_str}"
            )
        }
    }

    /// Update the anchor sizes
    fn update_anchor_sizes(&mut self, anchor: &Anchor) {
        self.anchor_sizes.insert(anchor.len());
    }

    /// Update the internal map with a new anchor and sequence
    fn update_internal(&mut self, anchor: Anchor, sequence: Sequence, index: usize) {
        self.map
            .entry(anchor)
            .or_insert_with(MapSequenceToIndex::default)
            .insert(sequence, index);
    }

    /// Insert a new anchor and sequence into the map
    pub fn insert(&mut self, anchor: Anchor, sequence: Sequence, index: usize) -> Result<()> {
        self.update_sequence_size(index, &sequence)?;
        self.update_anchor_sizes(&anchor);
        self.update_internal(anchor, sequence, index);
        Ok(())
    }

    /// Get the number of anchors in the map
    #[allow(dead_code)]
    pub fn num_anchors(&self) -> usize {
        self.map.len()
    }

    /// List all anchors in the map
    pub fn list_anchors(&self) -> Result<()> {
        let anchors = self.map.keys().collect::<Vec<_>>();
        for anchor in anchors {
            println!("{:?}", std::str::from_utf8(anchor)?);
        }
        Ok(())
    }

    /// Get the sequence map for a given anchor
    pub fn get_sequence_map(&self, anchor: &[u8]) -> Option<&MapSequenceToIndex> {
        self.map.get(anchor)
    }
}
