use super::{Alias, AliasNuc, Sequence};
use crate::io::utils::string_to_bytes;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Probe {
    #[serde(deserialize_with = "string_to_bytes")]
    pub sequence: Sequence,
    #[serde(deserialize_with = "string_to_bytes")]
    pub alias_nuc: AliasNuc,
    #[serde(deserialize_with = "string_to_bytes")]
    pub alias: Alias,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProbeAlias {
    pub nucleotides: AliasNuc,
    pub name: Alias,
}
impl ProbeAlias {
    pub fn new(nucleotides: AliasNuc, name: Alias) -> Self {
        Self { nucleotides, name }
    }
    pub fn name_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.name)
    }
    pub fn nucleotides_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.nucleotides)
    }
}
