use std::path::PathBuf;

use super::{Flex, Mapper};
use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    collection: Vec<Flex>,
}
impl Library {
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
    pub fn into_mapper(self) -> Result<Mapper> {
        Mapper::new(self)
    }
}
impl Iterator for Library {
    type Item = Flex;

    fn next(&mut self) -> Option<Self::Item> {
        self.collection.pop()
    }
}
