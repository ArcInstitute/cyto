use std::path::PathBuf;

use super::{Guide, Mapper};
use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    guides: Vec<Guide>,
}
impl Library {
    pub fn from_tsv(path: PathBuf) -> Result<Self> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(path)?;

        let guides = reader
            .deserialize()
            .map(|result| result.map_err(Into::into))
            .collect::<Result<Vec<Guide>>>()?;

        Ok(Self { guides })
    }
    pub fn into_mapper(self) -> Result<Mapper> {
        Mapper::new(self)
    }
}
impl Iterator for Library {
    type Item = Guide;

    fn next(&mut self) -> Option<Self::Item> {
        self.guides.pop()
    }
}
