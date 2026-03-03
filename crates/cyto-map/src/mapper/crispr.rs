use std::marker::PhantomData;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, bail};
use cyto_io::{FeatureWriter, match_input_transparent};
use log::{info, trace};
use seqhash::{MultiLenSeqHash, SeqHash, SeqHashBuilder};

use crate::geometry::ReadMate;
use crate::mapper::{Library, Mapper, Ready, Unpositioned};
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
    fn query(&self, seq: &[u8]) -> Option<usize> {
        let (mat, remap_offset) =
            self.anchor_hash
                .query_at_with_remap_offset(seq, self.anchor_pos, self.window)?;

        let protospacer_offset =
            ((self.anchor_pos + mat.seq_len()) as isize + remap_offset) as usize;

        self.protospacer_hash
            .query_at_with_remap(seq, protospacer_offset, self.window)
            .map(|m| m.parent_idx())
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
