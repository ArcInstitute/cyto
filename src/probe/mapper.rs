use anyhow::Result;

use super::{Library, MapSequenceToAlias, ProbeAlias};

#[derive(Debug)]
pub struct Mapper {
    sequence_to_alias: MapSequenceToAlias,
}
impl Mapper {
    pub fn new(probe_library: Library) -> Result<Self> {
        let mut sequence_to_alias = MapSequenceToAlias::default();
        probe_library
            .into_iter()
            .map(|probe| {
                sequence_to_alias.insert(probe.sequence, probe.alias_nuc, probe.alias)?;
                Ok(())
            })
            .collect::<Result<()>>()?;
        Ok(Self { sequence_to_alias })
    }

    pub fn map(&self, sequence: &[u8], offset: usize) -> Option<&ProbeAlias> {
        let rpos = offset;
        let lpos = rpos - self.sequence_to_alias.sequence_size;
        let subsequence = &sequence[lpos..rpos];
        self.sequence_to_alias.get(subsequence)
    }
}
