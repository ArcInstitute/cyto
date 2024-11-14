use anyhow::{bail, Result};
use hashbrown::HashMap;

use crate::aliases::{Name, SeqRef, Sequence};

#[derive(Default, Debug, Clone)]
pub struct MapSequenceToIndex {
    map: HashMap<Sequence, usize>,
    pub sequence_size: usize,
}
impl MapSequenceToIndex {
    fn update_sequence_size(&mut self, sequence: &Sequence) -> Result<()> {
        if self.sequence_size == 0 || self.sequence_size == sequence.len() {
            self.sequence_size = sequence.len();
            Ok(())
        } else {
            let sequence_str = std::str::from_utf8(sequence)?;
            let expected_size = self.sequence_size;
            let observed_size = sequence.len();
            bail!(
                "Probe sequence size mismatch\nExpected size: {expected_size}\nFound size: {observed_size}\nSequence: {sequence_str}"
            )
        }
    }

    /// Insert a sequence-alias pairing into the map
    pub fn insert(&mut self, sequence: Sequence, index: usize) -> Result<()> {
        self.update_sequence_size(&sequence)?;
        self.map.insert(sequence, index);
        Ok(())
    }

    /// Get a probe alias from the map given a sequence
    pub fn get(&self, sequence: SeqRef) -> Option<usize> {
        self.map.get(sequence).copied()
    }

    /// Get the length of the map
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MapIndexToName {
    map: HashMap<usize, Name>,
}
impl MapIndexToName {
    /// Insert an index-alias pairing into the map
    pub fn insert(&mut self, index: usize, name: Name) {
        self.map.insert(index, name);
    }

    /// Get an alias by index
    pub fn get(&self, index: usize) -> Option<&Name> {
        self.map.get(&index)
    }
}
