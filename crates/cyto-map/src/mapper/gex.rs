use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::{FeatureWriter, match_input_transparent};
use log::{info, trace};
use seqhash::SplitSeqHash;

use crate::geometry::ReadMate;
use crate::mapper::{FeatureMatch, Library, Mapper, Ready, Unpositioned};
use crate::stats::LibraryStatistics;
use crate::{Bijection, Component, GEX_MAX_HDIST, ResolvedGeometry};

#[derive(serde::Deserialize)]
struct GexRecord {
    probe_name: String,
    gene_name: String,
    seq: String,
}

pub struct GexMapper<S = Ready> {
    split_hash: SplitSeqHash,
    probe_names: Vec<String>,
    gene_names: Vec<String>,
    pos: usize,
    mate: ReadMate,
    init_time: f64,
    window: usize,
    _state: PhantomData<S>,
}

impl GexMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P, window: usize) -> Result<Self> {
        let start = Instant::now();
        let ihandle = match_input_transparent(Some(path))?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_reader(ihandle);

        let mut probe_names = Vec::default();
        let mut gene_names = Vec::default();
        let mut sequences = Vec::default();

        for result in reader.deserialize() {
            let record: GexRecord = result?;
            probe_names.push(record.probe_name);
            gene_names.push(record.gene_name);
            sequences.push(record.seq);
        }

        trace!("[GEX seqhash] - Starting build");
        let split_hash = SplitSeqHash::new(&sequences)?;
        let init_time = start.elapsed().as_secs_f64();
        info!(
            "[GEX seqhash] - Build complete ({:.2} ms)",
            init_time * 1000.0
        );

        Ok(Self {
            split_hash,
            gene_names,
            probe_names,
            pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
            window,
            init_time,
        })
    }

    /// Returns the sequence length of GEX probes in this mapper.
    pub fn seq_len(&self) -> usize {
        self.split_hash.seq_len()
    }

    /// Finalize the mapper with a position and read mate.
    pub fn with_position(self, pos: usize, mate: ReadMate) -> GexMapper<Ready> {
        GexMapper {
            split_hash: self.split_hash,
            probe_names: self.probe_names,
            gene_names: self.gene_names,
            pos,
            mate,
            init_time: self.init_time,
            _state: PhantomData,
            window: self.window,
        }
    }

    /// Scan all positions in `seq` for GEX probe matches, returning matched positions.
    /// Only positions where both halves of the split hash agree are included.
    pub fn scan_positions(&self, seq: &[u8]) -> Vec<usize> {
        self.split_hash
            .query_sliding_iter(seq)
            .filter(|(m, _)| m.agreed_idx().is_some())
            .map(|(_, pos)| pos)
            .collect()
    }

    pub fn resolve(self, geometry: &ResolvedGeometry) -> Result<GexMapper<Ready>> {
        let Some(region) = geometry.get(Component::Gex) else {
            bail!("geometry missing [gex]")
        };
        Ok(self.with_position(region.offset, region.mate))
    }
}

impl Mapper for GexMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<FeatureMatch> {
        let mat = self
            .split_hash
            .query_at_with_remap(seq, self.pos, self.window);

        let feature_idx = if mat.agreed_idx().is_some() {
            mat.agreed_idx()
        } else if mat.is_conflicted() {
            None
        } else if let Some((p_idx, half)) = mat.single_match() {
            let rem = mat.remaining_hdist(GEX_MAX_HDIST).unwrap_or(0);
            self.split_hash
                .is_within_hdist(seq, p_idx, half.other(), rem)
        } else {
            None
        }?;

        Some(FeatureMatch {
            feature_idx,
            end_pos: self.pos + self.split_hash.seq_len(),
        })
    }

    fn mate(&self) -> ReadMate {
        self.mate
    }
}

impl Library for GexMapper<Ready> {
    fn statistics(&self) -> LibraryStatistics {
        let biject = Bijection::new(&self.gene_names);
        LibraryStatistics {
            name: "gex",
            total_elem: self.split_hash.num_parents(),
            total_aggr: biject.len(),
            total_hash: self.split_hash.num_entries(),
            position: self.pos,
            mate: self.mate,
            init_time: self.init_time,
            exact: false,
            window: self.window,
        }
    }
}

impl<'a, T> FeatureWriter<'a> for GexMapper<T> {
    type Record = (&'a str, &'a str);

    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record> {
        self.probe_names
            .iter()
            .zip(self.gene_names.iter())
            .map(|(probe_name, gene_name)| (probe_name.as_str(), gene_name.as_str()))
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
    fn test_scan_positions_finds_gex_probe() {
        let gex_path = workspace_root().join("data/libraries/gex_probes.tsv");
        let mapper = GexMapper::from_file(&gex_path, 1).unwrap();

        assert_eq!(mapper.seq_len(), 50);

        // First probe sequence from gex_probes.tsv
        let probe_seq = b"GGTGACACCACAACAATGCAACGTATTTTGGATCTTGTCTACTGCATGGC";

        // Embed at position 0
        let mut read = probe_seq.to_vec();
        read.extend_from_slice(b"NNNNNNNNNN"); // padding
        let positions = mapper.scan_positions(&read);
        assert!(
            positions.contains(&0),
            "expected GEX probe match at position 0, got: {positions:?}"
        );

        // Embed at position 10
        let mut read2 = b"NNNNNNNNNN".to_vec();
        read2.extend_from_slice(probe_seq);
        read2.extend_from_slice(b"NNNNNNNNNN");
        let positions2 = mapper.scan_positions(&read2);
        assert!(
            positions2.contains(&10),
            "expected GEX probe match at position 10, got: {positions2:?}"
        );
    }

    #[test]
    fn test_scan_positions_no_match_on_random_seq() {
        let gex_path = workspace_root().join("data/libraries/gex_probes.tsv");
        let mapper = GexMapper::from_file(&gex_path, 1).unwrap();

        // Random sequence should not match
        let random_read = vec![b'N'; 100];
        let positions = mapper.scan_positions(&random_read);
        assert!(
            positions.is_empty(),
            "expected no matches on random sequence, got: {positions:?}"
        );
    }
}
