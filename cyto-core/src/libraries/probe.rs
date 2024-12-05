use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{mappers::ProbeMapper, metadata::Probe};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProbeLibrary {
    probes: Vec<Probe>,
}
impl ProbeLibrary {
    pub fn from_tsv(path: PathBuf) -> Result<Self> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(path)?;

        let probes = reader
            .deserialize()
            .map(|result| result.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { probes })
    }
    pub fn into_mapper(self) -> Result<ProbeMapper> {
        ProbeMapper::new(self)
    }
}
impl IntoIterator for ProbeLibrary {
    type Item = Probe;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.probes.into_iter()
    }
}
