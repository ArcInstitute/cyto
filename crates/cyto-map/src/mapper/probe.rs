use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::match_input_transparent;
use log::{info, trace};
use regex::Regex;
use seqhash::{SeqHash, SeqHashBuilder};

use crate::geometry::ReadMate;
use crate::mapper::{Bijection, FeatureMatch, Library, Mapper, Ready, Unpositioned};
use crate::stats::LibraryStatistics;
use crate::{Component, ResolvedGeometry};

#[derive(serde::Deserialize)]
struct ProbeRecord {
    seq: String,
    _nuc: String,
    alias: String,
}
pub struct ProbeMapper<S = Ready> {
    hash: SeqHash,
    aliases: Vec<String>,
    pos: usize,
    mate: ReadMate,
    init_time: f64,
    window: usize,
    exact: bool,
    /// When true, the probe follows a variable-length component in the geometry.
    /// Its offset is relative to the feature mapper's match end position, not
    /// absolute from read start. The processor must call `query_at()` with
    /// `FeatureMatch::end_pos + pos` instead of using `query()` directly.
    dynamic: bool,
    _state: PhantomData<S>,
}

impl ProbeMapper<Unpositioned> {
    /// Load probe sequences and aliases from a file, then build a `SeqHash`.
    pub fn from_file<P: AsRef<Path>>(path: P, exact: bool, window: usize) -> Result<Self> {
        let (sequences, aliases) = Self::load_from_file(path)?;
        Self::build(sequences, aliases, exact, window)
    }

    /// Load probe sequences and aliases from a file, filter aliases that match a regex, then build a `SeqHash`.
    pub fn from_file_with_alias_regex<P: AsRef<Path>>(
        path: P,
        exact: bool,
        window: usize,
        alias_regex: &str,
    ) -> Result<Self> {
        let regex = Regex::new(alias_regex)?;
        let (og_sequences, og_aliases) = Self::load_from_file(path)?;
        let num_og_sequences = og_sequences.len();
        let (sequences, aliases): (Vec<_>, Vec<_>) = og_sequences
            .into_iter()
            .zip(og_aliases)
            .filter(|(_, alias)| regex.is_match(alias))
            .unzip();
        trace!(
            "Kept {} of {} probe sequences ({:.2}%) after filtering regex: {}",
            sequences.len(),
            num_og_sequences,
            (sequences.len() as f64 / num_og_sequences as f64) * 100.0,
            alias_regex,
        );
        Self::build(sequences, aliases, exact, window)
    }

    /// Load probe sequences and aliases from a file.
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<(Vec<String>, Vec<String>)> {
        let ihandle = match_input_transparent(Some(path))?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_reader(ihandle);

        let mut sequences = Vec::new();
        let mut aliases = Vec::new();
        for record in reader.deserialize() {
            let record: ProbeRecord = record?;
            sequences.push(record.seq);
            aliases.push(record.alias);
        }

        Ok((sequences, aliases))
    }

    fn build(
        sequences: Vec<String>,
        aliases: Vec<String>,
        exact: bool,
        window: usize,
    ) -> Result<Self> {
        trace!("[PROBE seqhash] - Starting build");
        let start = Instant::now();
        let hash = if exact {
            SeqHashBuilder::default().exact().build(&sequences)
        } else {
            SeqHash::new(&sequences)
        }?;
        let init_time = start.elapsed().as_secs_f64();
        info!(
            "[PROBE seqhash] - Build complete ({:.2} ms)",
            init_time * 1000.0
        );
        Ok(Self {
            hash,
            aliases,
            pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
            window,
            exact,
            init_time,
            dynamic: false,
        })
    }

    /// Scan all positions in `seq` for probe matches, returning matched positions.
    pub fn scan_positions(&self, seq: &[u8]) -> Vec<usize> {
        self.hash
            .query_sliding_iter(seq)
            .map(|(_, pos)| pos)
            .collect()
    }

    /// Finalize the mapper with a position, read mate, and dynamic flag.
    pub fn with_position(self, pos: usize, mate: ReadMate, dynamic: bool) -> ProbeMapper<Ready> {
        ProbeMapper {
            hash: self.hash,
            aliases: self.aliases,
            pos,
            mate,
            _state: PhantomData,
            window: self.window,
            exact: self.exact,
            init_time: self.init_time,
            dynamic,
        }
    }

    pub fn resolve(self, geometry: &ResolvedGeometry) -> Result<ProbeMapper<Ready>> {
        let Some(region) = geometry.get(Component::Probe) else {
            bail!("geometry missing [probe]")
        };
        Ok(self.with_position(region.offset, region.mate, region.dynamic))
    }
}

impl<T> ProbeMapper<T> {
    /// Returns the sequence length of probes in this mapper.
    pub fn seq_len(&self) -> usize {
        self.hash.seq_len()
    }

    /// Returns the number of parent sequences used to make this mapper
    pub fn n_parents(&self) -> usize {
        self.hash.num_parents()
    }

    /// Returns the parent sequence for a given index
    pub fn get_parent(&self, idx: usize) -> Option<&String> {
        self.aliases.get(idx)
    }

    pub fn bijection(&self) -> Bijection<String> {
        Bijection::new(&self.aliases)
    }
}

impl ProbeMapper<Ready> {
    /// Returns true if this probe's offset is dynamic (follows a variable-length
    /// component in the geometry). When dynamic, the processor must use `query_at()`
    /// with `FeatureMatch::end_pos + dynamic_offset()` instead of `query()`.
    pub fn is_dynamic(&self) -> bool {
        self.dynamic
    }

    /// Returns the probe's offset relative to the feature match end position.
    /// Only meaningful when `is_dynamic()` is true.
    pub fn dynamic_offset(&self) -> usize {
        self.pos
    }

    /// Query at an explicit offset, used when the probe has a dynamic offset
    /// (i.e. it follows a variable-length component like anchor in the geometry).
    pub fn query_at(&self, seq: &[u8], offset: usize) -> Option<FeatureMatch> {
        self.hash
            .query_at_with_remap(seq, offset, self.window)
            .map(|m| FeatureMatch {
                feature_idx: m.parent_idx(),
                end_pos: offset + self.hash.seq_len(),
            })
    }
}

impl Mapper for ProbeMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<FeatureMatch> {
        self.query_at(seq, self.pos)
    }

    fn mate(&self) -> ReadMate {
        self.mate
    }
}

impl Library for ProbeMapper<Ready> {
    fn statistics(&self) -> LibraryStatistics {
        let biject = Bijection::new(&self.aliases);
        LibraryStatistics {
            name: "probe",
            total_elem: self.n_parents(),
            total_aggr: biject.len(),
            total_hash: self.hash.num_entries(),
            position: self.pos,
            mate: self.mate,
            init_time: self.init_time,
            window: self.window,
            exact: self.exact,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_scan_positions_finds_probe() {
        let probe_path =
            workspace_root().join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
        let mapper = ProbeMapper::from_file(&probe_path, true, 1).unwrap();

        assert_eq!(mapper.seq_len(), 8);

        // First probe sequence from probe-barcodes file: "ACTTTAGG"
        let probe_seq = b"ACTTTAGG";

        // Embed at position 0
        let mut read = probe_seq.to_vec();
        read.extend_from_slice(b"NNNNNNNNNNNNNNNN");
        let positions = mapper.scan_positions(&read);
        assert!(
            positions.contains(&0),
            "expected probe match at position 0, got: {positions:?}"
        );

        // Embed at position 20
        let mut read2 = b"NNNNNNNNNNNNNNNNNNNN".to_vec();
        read2.extend_from_slice(probe_seq);
        read2.extend_from_slice(b"NNNNNNNNNNNNNNNN");
        let positions2 = mapper.scan_positions(&read2);
        assert!(
            positions2.contains(&20),
            "expected probe match at position 20, got: {positions2:?}"
        );
    }

    #[test]
    fn test_scan_positions_no_match_on_random_seq() {
        let probe_path =
            workspace_root().join("data/metadata/probe-barcodes-fixed-rna-profiling.txt");
        let mapper = ProbeMapper::from_file(&probe_path, true, 1).unwrap();

        let random_read = vec![b'N'; 40];
        let positions = mapper.scan_positions(&random_read);
        assert!(
            positions.is_empty(),
            "expected no matches on random sequence, got: {positions:?}"
        );
    }
}
