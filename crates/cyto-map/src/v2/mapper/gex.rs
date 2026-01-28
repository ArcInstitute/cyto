use std::marker::PhantomData;
use std::path::Path;

use anyhow::Result;
use cyto_io::match_input_transparent;
use seqhash::SplitSeqHash;

use crate::v2::GEX_MAX_HDIST;
use crate::v2::geometry::ReadMate;
use crate::v2::mapper::{Mapper, Ready, Unpositioned};

#[derive(serde::Deserialize)]
struct GexRecord {
    probe_name: String,
    gene_name: String,
    seq: String,
}

pub struct GexMapper<S = Ready> {
    split_hash: SplitSeqHash,
    _probe_names: Vec<String>,
    _gene_names: Vec<String>,
    pos: usize,
    mate: ReadMate,
    _state: PhantomData<S>,
}

impl GexMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
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

        let split_hash = SplitSeqHash::new(&sequences)?;

        Ok(Self {
            split_hash,
            _probe_names: probe_names,
            _gene_names: gene_names,
            pos: 0,
            mate: ReadMate::R1,
            _state: PhantomData,
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
            _probe_names: self._probe_names,
            _gene_names: self._gene_names,
            pos,
            mate,
            _state: PhantomData,
        }
    }
}

impl Mapper for GexMapper<Ready> {
    fn query(&self, seq: &[u8]) -> Option<usize> {
        let mat = self.split_hash.query_at(seq, self.pos);

        if mat.agreed_idx().is_some() {
            mat.agreed_idx()
        } else if mat.is_conflicted() {
            None
        } else if let Some((p_idx, half)) = mat.single_match() {
            let rem = mat.remaining_hdist(GEX_MAX_HDIST).unwrap_or(0);
            self.split_hash
                .is_within_hdist(seq, p_idx, half.other(), rem)
                .then_some(p_idx)
        } else {
            None
        }
    }

    fn mate(&self) -> ReadMate {
        self.mate
    }
}
