use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use cyto_io::match_input_transparent;
use log::trace;
use seqhash::{SeqHash, SeqHashBuilder};

use crate::v2::REMAP_WINDOW;
use crate::v2::geometry::ReadMate;
use crate::v2::mapper::{Library, Mapper, Ready, Unpositioned};
use crate::v2::stats::LibraryStatistics;

#[derive(serde::Deserialize)]
struct Whitelist {
    seq: String,
}

pub struct WhitelistMapper<S = Ready> {
    hash: SeqHash,
    pos: usize,
    mate: ReadMate,
    init_time: f64,
    _state: PhantomData<S>,
}

impl WhitelistMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P, threads: usize) -> Result<Self> {
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
        let hash = SeqHashBuilder::default()
            .threads(threads)
            .build(&sequences)?;
        let init_time = start.elapsed().as_secs_f64();
        trace!(
            "[WHITELIST seqhash] - Build complete ({:.2} ms)",
            init_time * 1000.0
        );

        Ok(Self {
            hash,
            pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
            init_time,
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
        }
    }
}

impl<T> WhitelistMapper<T> {
    pub fn get_parent(&self, idx: usize) -> Option<&[u8]> {
        self.hash.get_parent(idx)
    }

    pub fn get_parent_2bit(&self, idx: usize) -> Option<Result<u64, bitnuc::Error>> {
        self.hash
            .get_parent(idx)
            .map(|seq| bitnuc::twobit::as_2bit_lossy(seq))
    }
}

impl Mapper for WhitelistMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<usize> {
        self.hash
            .query_at_with_remap(seq, self.pos, REMAP_WINDOW)
            .map(|m| m.parent_idx())
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
        }
    }
}
