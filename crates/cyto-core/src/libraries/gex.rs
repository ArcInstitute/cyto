use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{mappers::GexMapper, metadata::Gex};

#[derive(Debug, Serialize, Deserialize)]
pub struct GexLibrary {
    collection: Vec<Gex>,
}
impl GexLibrary {
    pub fn from_tsv(path: PathBuf) -> Result<Self> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(path)?;

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
