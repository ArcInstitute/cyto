use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::{FeatureWriter, match_input_transparent};
use log::{info, trace};
use seqhash::{MultiLenSeqHash, MultiLenSeqHashBuilder};

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
    protospacer_hash: MultiLenSeqHash,
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
            MultiLenSeqHashBuilder::default()
                .exact()
                .build(&protospacers)
        } else {
            MultiLenSeqHash::new(&protospacers)
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

    /// Returns the median sequence length
    pub fn protospacer_len(&self) -> usize {
        let mut lengths: Vec<usize> = self.protospacer_hash.lengths().collect();
        lengths.sort_unstable();
        let num_lengths: Vec<usize> = lengths
            .iter()
            .map(|&len| self.protospacer_hash.num_parents_for_len(len).unwrap())
            .collect();
        let total: usize = num_lengths.iter().sum();
        let median_idx = total / 2;
        let mut cumulative = 0;
        for (len, &count) in lengths.iter().zip(num_lengths.iter()) {
            cumulative += count;
            if cumulative >= median_idx {
                return *len;
            }
        }
        unreachable!("Should have found a median length")
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
            .query_at_with_remap_offset(seq, protospacer_offset, self.window)
            .map(|(m, remap_offset)| FeatureMatch {
                feature_idx: m.parent_idx(),
                end_pos: ((protospacer_offset + m.seq_len()) as isize + remap_offset) as usize,
            })
    }

    fn mate(&self) -> ReadMate {
        self.mate
    }
}

impl Library for CrisprMapper<Ready> {
    fn statistics(&self) -> LibraryStatistics {
        let total_hash = self
            .protospacer_hash
            .lengths()
            .map(|x| self.protospacer_hash.num_parents_for_len(x).unwrap())
            .sum();
        LibraryStatistics {
            name: "crispr",
            total_elem: self.protospacer_hash.num_parents(),
            total_aggr: self.protospacer_hash.num_parents(),
            total_hash,
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
    use rand::{Rng, SeedableRng, rngs::SmallRng, seq::IndexedRandom};
    use tempfile::NamedTempFile;

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

        assert_eq!(mapper.protospacer_hash.num_lengths(), 1);
        assert_eq!(mapper.protospacer_hash.lengths().next(), Some(20));

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

    fn gen_sequence<R: Rng>(rng: &mut R, length: usize) -> Vec<u8> {
        (0..length)
            .map(|_| *"ACGT".as_bytes().choose(rng).unwrap())
            .collect()
    }

    fn gen_x_unique_sequences<R: Rng>(rng: &mut R, length: usize, count: usize) -> Vec<Vec<u8>> {
        let mut sequences = std::collections::HashSet::new();
        while sequences.len() < count {
            let seq = gen_sequence(rng, length);
            sequences.insert(seq);
        }
        sequences.into_iter().collect()
    }

    #[test]
    fn test_protospacer_len() {
        let n_anchors = 2;
        let lengths = [17, 18, 19, 20];
        let totals = [100, 200, 400, 300];
        let mut rng = SmallRng::seed_from_u64(42);

        // generate anchors
        let anchors = gen_x_unique_sequences(&mut rng, 16, n_anchors);

        // generate protospacers with specified lengths and counts
        let mut records = Vec::default();
        let mut idx = 0;
        for (len, &count) in lengths.iter().zip(totals.iter()) {
            let sequences = gen_x_unique_sequences(&mut rng, *len, count);
            for seq in sequences {
                let anchor = &anchors[idx % n_anchors];
                let name = format!("record_{idx}");
                records.push(format!(
                    "{}\t{}\t{}",
                    name,
                    String::from_utf8_lossy(anchor),
                    String::from_utf8_lossy(&seq)
                ));
                idx += 1;
            }
        }

        // write to temp file
        let ntf = NamedTempFile::new().unwrap();
        std::fs::write(ntf.path(), records.join("\n")).unwrap();

        // create mapper from temp file
        let mapper = CrisprMapper::from_file(ntf.path(), false, 1).unwrap();

        let median_len = mapper.protospacer_len();
        assert_eq!(
            median_len, 19,
            "Expected median length 19, got {median_len}"
        );

        let num_protospacers = mapper.protospacer_hash.num_parents();
        assert_eq!(
            num_protospacers, 1000,
            "Expected 1000 protospacers, got {num_protospacers}"
        );

        let num_lengths = mapper.protospacer_hash.num_lengths();
        assert_eq!(
            num_lengths, 4,
            "Expected 4 unique lengths, got {num_lengths}"
        );
    }
}
