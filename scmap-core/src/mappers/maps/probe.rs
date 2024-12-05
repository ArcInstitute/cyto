use anyhow::{bail, Result};
use hashbrown::HashMap;

use crate::{
    aliases::{Alias, AliasNuc, SeqRef, Sequence},
    metadata::ProbeAlias,
};

#[derive(Default, Debug, Clone)]
pub struct MapIndexToAlias {
    map: HashMap<usize, usize>,
    alias_map: HashMap<usize, ProbeAlias>,
}
impl MapIndexToAlias {
    /// Insert an index-alias pairing into the map
    pub fn insert(&mut self, index: usize, alias_nuc: AliasNuc, alias: Alias) {
        let probe_alias = ProbeAlias::new(alias_nuc, alias);
        let probe_alias_index = self.alias_map.len();
        self.map.insert(index, probe_alias_index);
        self.alias_map.insert(probe_alias_index, probe_alias);
    }

    /// Get an alias by index
    ///
    /// This is used to get the full alias struct from the map
    pub fn get(&self, index: usize) -> Option<&ProbeAlias> {
        self.map
            .get(&index)
            .and_then(|alias_index| self.alias_map.get(alias_index))
    }

    /// Get the index of an alias by index
    ///
    /// This is used to the unique index of an alias in the map
    pub fn get_index(&self, index: usize) -> Option<usize> {
        self.map.get(&index).copied()
    }

    /// The number of unique aliases in the map
    pub fn num_unique_aliases(&self) -> usize {
        self.alias_map.len()
    }
}

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
