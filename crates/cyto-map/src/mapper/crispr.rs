use std::collections::HashMap;
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
    /// For each protospacer parent index (into `protospacer_hash`), the guides that
    /// carry that protospacer as `(anchor_global_idx, feature_idx)`. Length 1 for the
    /// common case; >1 only when a protospacer is shared across anchors. `feature_idx`
    /// is the 0-based TSV row index; `anchor_global_idx` is the anchor's index in the
    /// deduped anchor slice, which equals the global `parent_idx` from `MultiLenSeqHash`.
    proto_guides: Vec<Vec<(u32, u32)>>,
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
        let mut anchors: Vec<String> = Vec::default();
        let mut proto_unique: Vec<String> = Vec::default();
        let mut proto_index: HashMap<String, usize> = HashMap::default();
        let mut proto_guides: Vec<Vec<(u32, u32)>> = Vec::default();

        for (row_idx, result) in reader.deserialize().enumerate() {
            let record: CrisprRecord = result?;

            // Anchor global index. Dedup by first-occurrence order, which equals the
            // global `parent_idx` returned by `MultiLenSeqHash` (built from `anchors`),
            // even when anchors span multiple length groups. Anchor count is tiny (2 in
            // all known libraries), so a linear scan is fine.
            let anchor_idx = if let Some(i) = anchors.iter().position(|a| a == &record.anchor) {
                i
            } else {
                anchors.push(record.anchor.clone());
                anchors.len() - 1
            };

            // Protospacer parent index. Dedup by first-occurrence order, which equals the
            // `parent_idx` returned by the protospacer `SeqHash` (built from `proto_unique`).
            let proto_idx = if let Some(&i) = proto_index.get(&record.protospacer) {
                i
            } else {
                let i = proto_unique.len();
                proto_unique.push(record.protospacer.clone());
                proto_index.insert(record.protospacer.clone(), i);
                proto_guides.push(Vec::new());
                i
            };

            // Reject genuinely unresolvable duplicates: same anchor AND same protospacer.
            // Name both guides + rows so the error is actionable.
            if let Some(&(_, earlier_row)) = proto_guides[proto_idx]
                .iter()
                .find(|&&(a, _)| a as usize == anchor_idx)
            {
                bail!(
                    "guide '{}' (row {}) shares both anchor and protospacer with guide '{}' (row {}); \
                     cyto cannot distinguish guides identical in both anchor and protospacer",
                    record.name,
                    row_idx,
                    names[earlier_row as usize],
                    earlier_row
                );
            }
            proto_guides[proto_idx].push((anchor_idx as u32, row_idx as u32));

            names.push(record.name);
        }

        trace!("[CRISPR seqhash] - Starting build");
        let anchor_hash = MultiLenSeqHash::new(&anchors)?;
        let protospacer_hash = if exact {
            SeqHashBuilder::default().exact().build(&proto_unique)
        } else {
            SeqHash::new(&proto_unique)
        }?;

        // Surface the previously-crashing condition: a protospacer shared across anchors.
        let shared = proto_guides.iter().filter(|g| g.len() > 1).count();
        if shared > 0 {
            info!(
                "[CRISPR seqhash] - {shared} protospacer(s) shared across anchors; disambiguating by anchor"
            );
        }

        let init_time = start.elapsed().as_secs_f64();
        info!(
            "[CRISPR seqhash] - Build complete ({:.2} ms)",
            init_time * 1000.0
        );

        Ok(Self {
            anchor_hash,
            protospacer_hash,
            names,
            proto_guides,
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
            proto_guides: self.proto_guides,
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
        let anchor_idx = mat.parent_idx() as u32;

        let protospacer_offset =
            ((self.anchor_pos + mat.seq_len()) as isize + remap_offset) as usize;

        let m = self
            .protospacer_hash
            .query_at_with_remap(seq, protospacer_offset, self.window)?;

        let guides = &self.proto_guides[m.parent_idx()];
        let feature_idx = if guides.len() == 1 {
            // unique protospacer: ignore the anchor (backward-compatible)
            guides[0].1 as usize
        } else {
            // shared protospacer: attribute to the guide whose anchor matched;
            // unmapped if the matched anchor carries none of these guides
            guides.iter().find(|&&(a, _)| a == anchor_idx)?.1 as usize
        };

        Some(FeatureMatch {
            feature_idx,
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
            total_elem: self.names.len(),
            total_aggr: self.names.len(),
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

    // --- (anchor, protospacer) disambiguate-on-collision keying tests ---

    // Anchors A and B are the real library anchors (A = 33bp, row 0 of crispr_guides.tsv;
    // B = 30bp), written verbatim into crispr_dup_anchor.tsv. C is a synthetic distinct
    // 30bp anchor carrying none of the shared protospacer's guides (for the None path).
    const ANCHOR_A: &[u8] = b"CTTGCTATGCACTCTTGTGCTTAGCTCTGAAAC"; // 33bp
    const ANCHOR_B: &[u8] = b"GCTATGCTGTTTCCAGCTTAGCTCTTAAAC"; // 30bp
    const ANCHOR_C: &[u8] = b"AACCGGTTAACCGGTTAACCGGTTAACCGG"; // 30bp
    const PROTO_SHARED: &[u8] = b"AAAACCCCGGGGTTTTACGT";
    const PROTO_UNIQ_A: &[u8] = b"CACGTACGTACGTACGTACG";

    fn dup_anchor_path() -> std::path::PathBuf {
        workspace_root().join("data/libraries/crispr_dup_anchor.tsv")
    }

    /// Build a `Ready` mapper with the anchor anchored at position 0, window 1.
    fn ready(path: &std::path::Path, exact: bool) -> CrisprMapper<Ready> {
        CrisprMapper::from_file(path, exact, 1)
            .unwrap()
            .with_position(0, ReadMate::R1)
    }

    /// Concatenate `anchor ++ protospacer` into a synthetic read. The anchor sits at
    /// position 0; `query` recomputes the protospacer offset from the matched anchor's
    /// length, so anchors of different lengths place the protospacer correctly.
    /// `anchor_pos == 0` is safe: `query_at` bounds-checks `pos + seq_len <= seq.len()`.
    fn read_of(anchor: &[u8], protospacer: &[u8]) -> Vec<u8> {
        let mut r = anchor.to_vec();
        r.extend_from_slice(protospacer);
        r
    }

    #[test]
    fn test_collision_resolves_by_anchor() {
        let mapper = ready(&dup_anchor_path(), false);
        // Shared protospacer under anchor A -> row 0 (SHARED_A); under anchor B -> row 1 (SHARED_B).
        // Assert the exact expected row index for each read individually, not merely that they differ.
        assert_eq!(
            mapper
                .query(&read_of(ANCHOR_A, PROTO_SHARED))
                .unwrap()
                .feature_idx,
            0
        );
        assert_eq!(
            mapper
                .query(&read_of(ANCHOR_B, PROTO_SHARED))
                .unwrap()
                .feature_idx,
            1
        );
    }

    #[test]
    fn test_unique_protospacer_ignores_anchor() {
        let mapper = ready(&dup_anchor_path(), false);
        // UNIQ_A's protospacer is unique; paired with the "wrong" anchor B it still maps
        // to UNIQ_A's row (2). Anchor is ignored when the protospacer is unique (backward-compat).
        assert_eq!(
            mapper
                .query(&read_of(ANCHOR_B, PROTO_UNIQ_A))
                .unwrap()
                .feature_idx,
            2
        );
    }

    #[test]
    fn test_collision_wrong_anchor_returns_none() {
        let mapper = ready(&dup_anchor_path(), false);
        // Anchor C carries none of the shared protospacer's guides -> unmapped (None).
        assert!(mapper.query(&read_of(ANCHOR_C, PROTO_SHARED)).is_none());
    }

    #[test]
    fn test_duplicate_anchor_protospacer_pair_errors() {
        let path = workspace_root().join("data/libraries/crispr_dup_pair.tsv");
        // `.err()` drops the Ok value (CrisprMapper is not Debug, so `unwrap_err` won't compile).
        let err = CrisprMapper::from_file(&path, false, 1)
            .err()
            .expect("duplicate (anchor, protospacer) pair should be rejected");
        let msg = err.to_string();
        assert!(msg.contains("DUPE_A"), "error should name DUPE_A: {msg}");
        assert!(msg.contains("DUPE_B"), "error should name DUPE_B: {msg}");
    }

    #[test]
    fn test_statistics_and_record_stream_row_count() {
        let mapper = ready(&dup_anchor_path(), false);
        let stats = mapper.statistics();
        // 4 rows / 4 (anchor, protospacer) pairs, even though only 3 unique protospacers.
        assert_eq!(stats.total_elem, 4);
        assert_eq!(stats.total_aggr, 4);
        // record_stream emits names in row order, aligned 1:1 with feature_idx.
        assert_eq!(mapper.record_stream().count(), 4);
        assert_eq!(mapper.record_stream().next().unwrap().0, "SHARED_A");
        assert_eq!(mapper.record_stream().nth(2).unwrap().0, "UNIQ_A");
    }

    #[test]
    fn test_exact_build_on_collision() {
        // exact=true takes the SeqHashBuilder::exact() branch; dedup still applies.
        let mapper = ready(&dup_anchor_path(), true);
        assert_eq!(
            mapper
                .query(&read_of(ANCHOR_A, PROTO_SHARED))
                .unwrap()
                .feature_idx,
            0
        );
    }

    #[test]
    fn test_backward_compat_real_library() {
        let path = workspace_root().join("data/libraries/crispr_guides.tsv");
        let mapper = ready(&path, false);
        // Row 0: anchor A (33bp) + unique protospacer CACTCCACGTCGCCCGGAGC -> feature_idx 0.
        let read = read_of(ANCHOR_A, b"CACTCCACGTCGCCCGGAGC");
        assert_eq!(mapper.query(&read).unwrap().feature_idx, 0);
    }
}
