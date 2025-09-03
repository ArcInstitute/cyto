use std::path::Path;

use anyhow::{bail, Context, Result};
use csv::ReaderBuilder;
use log::{debug, error};
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};

use crate::{mappers::ProbeMapper, metadata::Probe};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProbeLibrary {
    probes: Vec<Probe>,
}
impl ProbeLibrary {
    pub fn from_tsv<P: AsRef<Path>>(path: P, pattern: Option<&str>) -> Result<Self> {
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

        let alias_re = match pattern {
            // match the required pattern
            Some(re) => {
                debug!("Building probe regex from pattern: {}", re);
                Regex::new(re)?
            }
            // match everything if no regex is provided
            None => Regex::new(r"^[A-Z0-9]+$")?,
        };

        let mut probes = Vec::new();
        let mut excluded = 0;
        for result in reader.deserialize() {
            let probe: Probe = result?;
            if alias_re.is_match(&probe.alias) {
                probes.push(probe);
            } else {
                excluded += 1;
            }
        }

        if probes.is_empty() {
            if let Some(pattern) = pattern {
                bail!("No probes found matching pattern: {}", pattern);
            } else {
                bail!("No probes found");
            }
        } else {
            if let Some(pattern) = pattern {
                if excluded > 0 {
                    debug!(
                        "Included {} of {} probes matching pattern: {}",
                        probes.len(),
                        probes.len() + excluded,
                        pattern
                    );
                } else {
                    debug!("All probes matched pattern: {}", pattern);
                }
            }
        }

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
