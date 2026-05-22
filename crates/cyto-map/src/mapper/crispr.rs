use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::{FeatureWriter, match_input_transparent};
use log::{info, trace};
use seqhash::{MultiLenSeqHash, SeqHash, SeqHashBuilder};

use crate::geometry::ReadMate;
use crate::mapper::{FeatureMatch, Library, Mapper, Ready, Unpositioned};
use crate::stats::LibraryStatistics;
use crate::{Component, ResolvedGeometry};

#[derive(serde::Deserialize)]
struct CrisprRecord {
    name: String,
    anchor: String,
    protospacer: String,
}

pub struct CrisprMapper<S = Ready> {
    anchor_hash: MultiLenSeqHash,
    protospacer_hash: SeqHash,
    names: Vec<String>,
    anchor_pos: usize,
    mate: ReadMate,
    init_time: f64,
    window: usize,
    exact: bool,
    _state: PhantomData<S>,
}

impl CrisprMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P, exact: bool, window: usize) -> Result<Self> {
        let start = Instant::now();
        let ihandle = match_input_transparent(Some(path))?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_reader(ihandle);

        let mut names = Vec::default();
        let mut anchors = Vec::default();
        let mut protospacers = Vec::default();

        for result in reader.deserialize() {
            let record: CrisprRecord = result?;
            names.push(record.name);

            if !anchors.contains(&record.anchor) {
                anchors.push(record.anchor);
            }

            protospacers.push(record.protospacer);
        }

        trace!("[CRISPR seqhash] - Starting build");
        let anchor_hash = MultiLenSeqHash::new(&anchors)?;
        let protospacer_hash = if exact {
            SeqHashBuilder::default().exact().build(&protospacers)
        } else {
            SeqHash::new(&protospacers)
        }?;
        let init_time = start.elapsed().as_secs_f64();
        info!(
            "[CRISPR seqhash] - Build complete ({:.2} ms)",
            init_time * 1000.0
        );

        Ok(Self {
            anchor_hash,
            protospacer_hash,
            names,
            anchor_pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
            init_time,
            window,
            exact,
        })
    }

    /// Returns the sequence length of protospacers.
    pub fn protospacer_len(&self) -> usize {
        self.protospacer_hash.seq_len()
    }

    /// Anchor is variable length, returns None.
    pub fn anchor_len(&self) -> Option<usize> {
        None
    }

    /// Finalize the mapper with anchor position and read mate.
    /// Protospacer position is computed dynamically based on anchor match.
    pub fn with_position(self, anchor_pos: usize, mate: ReadMate) -> CrisprMapper<Ready> {
        CrisprMapper {
            anchor_hash: self.anchor_hash,
            protospacer_hash: self.protospacer_hash,
            names: self.names,
            anchor_pos,
            mate,
            init_time: self.init_time,
            _state: PhantomData,
            window: self.window,
            exact: self.exact,
        }
    }

    /// Scan all positions in `seq` for anchor matches, returning matched positions.
    pub fn scan_anchor_positions(&self, seq: &[u8]) -> Vec<usize> {
        self.anchor_hash
            .query_sliding_iter(seq)
            .map(|(_, pos)| pos)
            .collect()
    }

    /// Scan all positions in `seq` for protospacer matches, returning matched positions.
    pub fn scan_protospacer_positions(&self, seq: &[u8]) -> Vec<usize> {
        self.protospacer_hash
            .query_sliding_iter(seq)
            .map(|(_, pos)| pos)
            .collect()
    }

    pub fn resolve(self, geometry: &ResolvedGeometry) -> Result<CrisprMapper<Ready>> {
        let Some(anchor_region) = geometry.get(Component::Anchor) else {
            bail!("geometry missing [anchor]")
        };
        let Some(_protospacer_region) = geometry.get(Component::Protospacer) else {
            bail!("geometry missing [protospacer]")
        };
        Ok(self.with_position(anchor_region.offset, anchor_region.mate))
    }
}

impl Mapper for CrisprMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<FeatureMatch> {
        let (mat, remap_offset) =
            self.anchor_hash
                .query_at_with_remap_offset(seq, self.anchor_pos, self.window)?;

        let protospacer_offset =
            ((self.anchor_pos + mat.seq_len()) as isize + remap_offset) as usize;

        self.protospacer_hash
            .query_at_with_remap(seq, protospacer_offset, self.window)
            .map(|m| FeatureMatch {
                feature_idx: m.parent_idx(),
                end_pos: protospacer_offset + self.protospacer_hash.seq_len(),
            })
    }

    fn mate(&self) -> ReadMate {
        self.mate
    }
}

impl Library for CrisprMapper<Ready> {
    fn statistics(&self) -> LibraryStatistics {
        LibraryStatistics {
            name: "crispr",
            total_elem: self.protospacer_hash.num_parents(),
            total_aggr: self.protospacer_hash.num_parents(),
            total_hash: self.protospacer_hash.num_entries(),
            position: self.anchor_pos,
            mate: self.mate,
            init_time: self.init_time,
            exact: self.exact,
            window: self.window,
        }
    }
}

impl<'a, T> FeatureWriter<'a> for CrisprMapper<T> {
    type Record = (&'a str, &'a str);

    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record> {
        self.names.iter().map(|name| (name.as_str(), name.as_str()))
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
    fn test_scan_anchor_positions() {
        let guides_path = workspace_root().join("data/libraries/crispr_guides.tsv");
        let mapper = CrisprMapper::from_file(&guides_path, false, 1).unwrap();

        // First anchor from crispr_guides.tsv: "CTTGCTATGCACTCTTGTGCTTAGCTCTGAAAC" (33bp)
        let anchor_seq = b"CTTGCTATGCACTCTTGTGCTTAGCTCTGAAAC";

        // Embed at position 0
        let mut read = anchor_seq.to_vec();
        read.extend_from_slice(b"NNNNNNNNNNNNNNNNNNNN");
        let positions = mapper.scan_anchor_positions(&read);
        assert!(
            positions.contains(&0),
            "expected anchor match at position 0, got: {positions:?}"
        );

        // Embed at position 8
        let mut read2 = b"NNNNNNNN".to_vec();
        read2.extend_from_slice(anchor_seq);
        read2.extend_from_slice(b"NNNNNNNNNNNNNNNNNNNN");
        let positions2 = mapper.scan_anchor_positions(&read2);
        assert!(
            positions2.contains(&8),
            "expected anchor match at position 8, got: {positions2:?}"
        );
    }

    #[test]
    fn test_scan_protospacer_positions() {
        let guides_path = workspace_root().join("data/libraries/crispr_guides.tsv");
        let mapper = CrisprMapper::from_file(&guides_path, false, 1).unwrap();

        assert_eq!(mapper.protospacer_len(), 20);

        // First protospacer from crispr_guides.tsv: "CACTCCACGTCGCCCGGAGC" (20bp)
        let proto_seq = b"CACTCCACGTCGCCCGGAGC";

        // Embed at position 0
        let mut read = proto_seq.to_vec();
        read.extend_from_slice(b"NNNNNNNNNN");
        let positions = mapper.scan_protospacer_positions(&read);
        assert!(
            positions.contains(&0),
            "expected protospacer match at position 0, got: {positions:?}"
        );

        // Embed at position 15
        let mut read2 = b"NNNNNNNNNNNNNNN".to_vec();
        read2.extend_from_slice(proto_seq);
        read2.extend_from_slice(b"NNNNNNNNNN");
        let positions2 = mapper.scan_protospacer_positions(&read2);
        assert!(
            positions2.contains(&15),
            "expected protospacer match at position 15, got: {positions2:?}"
        );
    }

    #[test]
    fn test_scan_no_match_on_random_seq() {
        let guides_path = workspace_root().join("data/libraries/crispr_guides.tsv");
        let mapper = CrisprMapper::from_file(&guides_path, false, 1).unwrap();

        let random_read = vec![b'N'; 80];
        assert!(mapper.scan_anchor_positions(&random_read).is_empty());
        assert!(mapper.scan_protospacer_positions(&random_read).is_empty());
    }
}
