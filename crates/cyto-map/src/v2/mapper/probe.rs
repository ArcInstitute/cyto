use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use cyto_io::match_input_transparent;
use log::{info, trace};
use seqhash::SeqHash;

use crate::v2::REMAP_WINDOW;
use crate::v2::geometry::ReadMate;
use crate::v2::mapper::{Bijection, Library, Mapper, Ready, Unpositioned};
use crate::v2::stats::LibraryStatistics;

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
    _state: PhantomData<S>,
}

impl ProbeMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let start = Instant::now();
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

        trace!("[PROBE seqhash] - Starting build");
        let hash = SeqHash::new(&sequences)?;
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
            init_time,
        })
    }

    /// Finalize the mapper with a position and read mate.
    pub fn with_position(self, pos: usize, mate: ReadMate) -> ProbeMapper<Ready> {
        ProbeMapper {
            hash: self.hash,
            aliases: self.aliases,
            pos,
            mate,
            init_time: self.init_time,
            _state: PhantomData,
        }
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

impl Mapper for ProbeMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<usize> {
        self.hash
            .query_at_with_remap(seq, self.pos, REMAP_WINDOW)
            .map(|m| m.parent_idx())
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
        }
    }
}
