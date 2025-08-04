use std::path::Path;

use crate::{mappers::CrisprMapper, metadata::Guide};
use anyhow::{Context, Result};
use csv::ReaderBuilder;
use log::{debug, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CrisprLibrary {
    guides: Vec<Guide>,
}
impl CrisprLibrary {
    pub fn from_tsv<P: AsRef<Path>>(path: P) -> Result<Self> {
        debug!("Building CRISPR library from: {}", path.as_ref().display());
        if !path.as_ref().exists() {
            error!("Missing file: {}", path.as_ref().display());
        }
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(&path)
            .context(format!("Unable to open file {}", path.as_ref().display()))?;

        let guides = reader
            .deserialize()
            .map(|result| result.map_err(Into::into))
            .collect::<Result<Vec<Guide>>>()?;

        Ok(Self { guides })
    }
    pub fn into_mapper(self) -> Result<CrisprMapper> {
        CrisprMapper::new(self)
    }
    pub fn into_corrected_mapper(self) -> Result<CrisprMapper> {
        CrisprMapper::new_corrected(self)
    }
    pub fn len(&self) -> usize {
        self.guides.len()
    }
}
impl IntoIterator for CrisprLibrary {
    type Item = Guide;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.guides.into_iter()
    }
}
