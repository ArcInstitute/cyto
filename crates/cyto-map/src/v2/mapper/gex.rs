use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::{FeatureWriter, match_input_transparent};
use log::{info, trace};
use seqhash::SplitSeqHash;

use crate::v2::geometry::ReadMate;
use crate::v2::mapper::{Library, Mapper, Ready, Unpositioned};
use crate::v2::stats::LibraryStatistics;
use crate::v2::{Bijection, Component, GEX_MAX_HDIST, ResolvedGeometry};

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
    _state: PhantomData<S>,
}

impl GexMapper<Unpositioned> {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
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
        }
    }

    pub fn resolve(self, geometry: &ResolvedGeometry) -> Result<GexMapper<Ready>> {
        let Some(region) = geometry.get(Component::Gex) else {
            bail!("geometry missing [gex]")
        };
        Ok(self.with_position(region.offset, region.mate))
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
