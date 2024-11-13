use std::path::PathBuf;

use crate::{mappers::CrisprMapper, metadata::Guide};
use anyhow::Result;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CrisprLibrary {
    guides: Vec<Guide>,
}
impl CrisprLibrary {
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
    pub fn into_mapper(self) -> Result<CrisprMapper> {
        CrisprMapper::new(self)
    }
}
impl Iterator for CrisprLibrary {
    type Item = Guide;

    fn next(&mut self) -> Option<Self::Item> {
        self.guides.pop()
    }
}
