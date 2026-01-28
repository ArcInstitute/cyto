use std::marker::PhantomData;
use std::path::Path;

use anyhow::Result;
use cyto_io::match_input_transparent;
use seqhash::SeqHash;

use crate::v2::REMAP_WINDOW;
use crate::v2::geometry::ReadMate;
use crate::v2::mapper::{Bijection, Mapper, Ready, Unpositioned};

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
    _state: PhantomData<S>,
}

impl ProbeMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
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

        let hash = SeqHash::new(&sequences)?;

        Ok(Self {
            hash,
            aliases,
            pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
        })
    }

    /// Finalize the mapper with a position and read mate.
    pub fn with_position(self, pos: usize, mate: ReadMate) -> ProbeMapper<Ready> {
        ProbeMapper {
            hash: self.hash,
            aliases: self.aliases,
            pos,
            mate,
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
