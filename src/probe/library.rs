use std::path::PathBuf;

use super::{Mapper, Probe};
use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    probes: Vec<Probe>,
}
impl Library {
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
    pub fn into_mapper(self) -> Result<Mapper> {
        Mapper::new(self)
    }
}
impl Iterator for Library {
    type Item = Probe;

    fn next(&mut self) -> Option<Self::Item> {
        self.probes.pop()
    }
}
