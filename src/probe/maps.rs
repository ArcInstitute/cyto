use super::{Alias, AliasNuc, ProbeAlias, Sequence};
use anyhow::{bail, Result};
use hashbrown::HashMap;

#[derive(Default, Debug)]
pub struct MapSequenceToAlias {
    map: HashMap<Sequence, ProbeAlias>,
    pub sequence_size: usize,
}
impl MapSequenceToAlias {
    fn update_sequence_size(&mut self, sequence: &Sequence) -> Result<()> {
        if self.sequence_size == 0 || self.sequence_size == sequence.len() {
            self.sequence_size = sequence.len();
            Ok(())
        } else {
            let sequence_str = std::str::from_utf8(&sequence)?;
            let expected_size = self.sequence_size;
            let observed_size = sequence.len();
            bail!(
                "Probe sequence size mismatch\nExpected size: {expected_size}\nFound size: {observed_size}\nSequence: {sequence_str}"
            )
        }
    }

    /// Insert a sequence-alias pairing into the map
    pub fn insert(&mut self, sequence: Sequence, alias_nuc: AliasNuc, alias: Alias) -> Result<()> {
        self.update_sequence_size(&sequence)?;
        self.map.insert(sequence, ProbeAlias::new(alias_nuc, alias));
        Ok(())
    }

    /// Get a probe alias from the map given a sequence
    pub fn get(&self, sequence: &[u8]) -> Option<&ProbeAlias> {
        self.map.get(sequence)
    }
}
