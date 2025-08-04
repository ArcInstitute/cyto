use std::path::Path;

use anyhow::{Context, Result};
use csv::ReaderBuilder;
use log::{debug, error};
use serde::{Deserialize, Serialize};

use crate::{mappers::ProbeMapper, metadata::Probe};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProbeLibrary {
    probes: Vec<Probe>,
}
impl ProbeLibrary {
    pub fn from_tsv<P: AsRef<Path>>(path: P) -> Result<Self> {
        debug!(
            "Building Flex Demultiplexing Probe library from: {}",
            path.as_ref().display()
        );
        if !path.as_ref().exists() {
            error!("Missing file: {}", path.as_ref().display());
        }
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(&path)
            .context(format!("Unable to open file {}", path.as_ref().display()))?;

        let probes = reader
            .deserialize()
            .map(|result| result.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { probes })
    }
    pub fn into_mapper(self) -> Result<ProbeMapper> {
        ProbeMapper::new(self)
    }
    pub fn into_corrected_mapper(self) -> Result<ProbeMapper> {
        ProbeMapper::new_corrected(self)
    }
}
impl IntoIterator for ProbeLibrary {
    type Item = Probe;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.probes.into_iter()
    }
}
