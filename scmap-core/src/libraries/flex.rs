use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{mappers::FlexMapper, metadata::Flex};

#[derive(Debug, Serialize, Deserialize)]
pub struct FlexLibrary {
    collection: Vec<Flex>,
}
impl FlexLibrary {
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
    pub fn into_mapper(self) -> Result<FlexMapper> {
        FlexMapper::new(self)
    }
    pub fn len(&self) -> usize {
        self.collection.len()
    }
}
impl IntoIterator for FlexLibrary {
    type Item = Flex;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.collection.into_iter()
    }
}
