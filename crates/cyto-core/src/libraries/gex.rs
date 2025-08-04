use std::path::Path;

use anyhow::{Context, Result};
use csv::ReaderBuilder;
use log::{debug, error};
use serde::{Deserialize, Serialize};

use crate::{mappers::GexMapper, metadata::Gex};

#[derive(Debug, Serialize, Deserialize)]
pub struct GexLibrary {
    collection: Vec<Gex>,
}
impl GexLibrary {
    pub fn from_tsv<P: AsRef<Path>>(ref path: P) -> Result<Self> {
        debug!("Building GEX library from: {}", path.as_ref().display());
        if !path.as_ref().exists() {
            error!("Missing file: {}", path.as_ref().display());
        }
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(path)
            .context(format!("Unable to open file {}", path.as_ref().display()))?;

        let collection = reader
            .deserialize()
            .map(|result| result.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { collection })
    }
    pub fn into_mapper(self) -> Result<GexMapper> {
        GexMapper::new(self)
    }
    pub fn len(&self) -> usize {
        self.collection.len()
    }
}
impl IntoIterator for GexLibrary {
    type Item = Gex;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.collection.into_iter()
    }
}
