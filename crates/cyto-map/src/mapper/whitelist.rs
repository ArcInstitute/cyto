use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::match_input_transparent;
use log::{info, trace};
use seqhash::{SeqHash, SeqHashBuilder};

use crate::geometry::ReadMate;
use crate::mapper::{FeatureMatch, Library, Mapper, Ready, Unpositioned};
use crate::stats::LibraryStatistics;
use crate::{Component, ResolvedGeometry};

#[derive(serde::Deserialize)]
struct Whitelist {
    seq: String,
}

pub struct WhitelistMapper<S = Ready> {
    hash: SeqHash,
    pos: usize,
    mate: ReadMate,
    init_time: f64,
    exact: bool,
    window: usize,
    _state: PhantomData<S>,
}

impl WhitelistMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(
        path: P,
        exact: bool,
        window: usize,
        threads: usize,
    ) -> Result<Self> {
        let start = Instant::now();
        let ihandle = match_input_transparent(Some(path))?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_reader(ihandle);

        let mut sequences = Vec::new();
        for result in reader.deserialize() {
            let record: Whitelist = result?;
            sequences.push(record.seq);
        }

        trace!("[WHITELIST seqhash] - Starting build");
        let hash = if exact {
            SeqHashBuilder::default()
                .threads(threads)
                .exact()
                .build(&sequences)
        } else {
            SeqHashBuilder::default().threads(threads).build(&sequences)
        }?;
        let init_time = start.elapsed().as_secs_f64();
        info!(
            "[WHITELIST seqhash] - Build complete ({:.2} ms)",
            init_time * 1000.0
        );

        Ok(Self {
            hash,
            pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
            init_time,
            window,
            exact,
        })
    }

    /// Returns the sequence length of barcodes in this mapper.
    pub fn seq_len(&self) -> usize {
        self.hash.seq_len()
    }

    /// Finalize the mapper with a position and read mate.
    pub fn with_position(self, pos: usize, mate: ReadMate) -> WhitelistMapper<Ready> {
        WhitelistMapper {
            hash: self.hash,
            pos,
            mate,
            init_time: self.init_time,
            _state: PhantomData,
            window: self.window,
            exact: self.exact,
        }
    }

    /// Scan all positions in `seq` for barcode matches, returning matched positions.
    pub fn scan_positions(&self, seq: &[u8]) -> Vec<usize> {
        self.hash
            .query_sliding_iter(seq)
            .map(|(_, pos)| pos)
            .collect()
    }

    pub fn resolve(self, geometry: &ResolvedGeometry) -> Result<WhitelistMapper<Ready>> {
        let Some(region) = geometry.get(Component::Barcode) else {
            bail!("geometry missing [barcode]")
        };
        Ok(self.with_position(region.offset, region.mate))
    }
}

impl<T> WhitelistMapper<T> {
    pub fn get_parent(&self, idx: usize) -> Option<&[u8]> {
        self.hash.get_parent(idx)
    }

    pub fn get_parent_2bit(&self, idx: usize) -> Option<Result<u64, bitnuc::Error>> {
        self.hash.get_parent(idx).map(bitnuc::twobit::as_2bit_lossy)
    }
}

impl Mapper for WhitelistMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<FeatureMatch> {
        self.hash
            .query_at_with_remap(seq, self.pos, self.window)
            .map(|m| FeatureMatch {
                feature_idx: m.parent_idx(),
                end_pos: self.pos + self.hash.seq_len(),
            })
    }

    fn mate(&self) -> ReadMate {
        self.mate
    }
}

impl Library for WhitelistMapper<Ready> {
    fn statistics(&self) -> LibraryStatistics {
        LibraryStatistics {
            name: "whitelist",
            total_elem: self.hash.num_parents(),
            total_aggr: self.hash.num_parents(),
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
    fn test_scan_positions_finds_barcode() {
        let whitelist_path = workspace_root().join("data/metadata/737K-fixed-rna-profiling.txt.gz");
        let mapper = WhitelistMapper::from_file(&whitelist_path, true, 1, 1).unwrap();

        assert_eq!(mapper.seq_len(), 16);

        // Read the first barcode from the whitelist and embed it in a synthetic read
        let first_barcode = b"AAACAAGCAAACAAGA"; // first entry in the whitelist

        // Embed at position 0
        let mut read = first_barcode.to_vec();
        read.extend_from_slice(b"NNNNNNNNNNNN"); // padding
        let positions = mapper.scan_positions(&read);
        assert!(
            positions.contains(&0),
            "expected barcode match at position 0, got: {positions:?}"
        );

        // Embed at position 5
        let mut read2 = b"NNNNN".to_vec();
        read2.extend_from_slice(first_barcode);
        read2.extend_from_slice(b"NNNNNNNNNNNN");
        let positions2 = mapper.scan_positions(&read2);
        assert!(
            positions2.contains(&5),
            "expected barcode match at position 5, got: {positions2:?}"
        );
    }

    #[test]
    fn test_scan_positions_no_match_on_random_seq() {
        let whitelist_path = workspace_root().join("data/metadata/737K-fixed-rna-profiling.txt.gz");
        let mapper = WhitelistMapper::from_file(&whitelist_path, true, 1, 1).unwrap();

        // All Ns should not match any barcode
        let random_read = b"NNNNNNNNNNNNNNNNNNNNNNNNNNNN";
        let positions = mapper.scan_positions(random_read);
        assert!(
            positions.is_empty(),
            "expected no matches on random sequence, got: {positions:?}"
        );
    }
}
