use anyhow::{bail, Result};
use disambiseq::Disambibyte;
use hashbrown::HashMap;

use crate::aliases::{Name, SeqRef, Sequence};

const MAXIMUM_DISTANCE: usize = 3;

#[derive(Default, Debug, Clone)]
pub struct MapSequenceToIndex {
    /// Maps the full sequence to its index
    map: HashMap<Sequence, usize>,
    /// Maps the index to its full sequence
    seq_map: HashMap<usize, Sequence>,
    /// Maps the left half of the sequence to its index
    left_map: HashMap<Sequence, usize>,
    /// Maps the right half of the sequence to its index
    right_map: HashMap<Sequence, usize>,
    /// A correction for the left half of the sequence
    left_correction: Disambibyte,
    /// A correction for the right half of the sequence
    right_correction: Disambibyte,
    /// The size of the full sequence
    pub sequence_size: usize,
    /// The size of the left half of the sequence
    pub left_size: usize,
}
impl MapSequenceToIndex {
    fn update_sequence_size(&mut self, sequence: &Sequence) -> Result<()> {
        if self.sequence_size == 0 || self.sequence_size == sequence.len() {
            self.sequence_size = sequence.len();
            self.left_size = sequence.len() / 2;
            Ok(())
        } else {
            let sequence_str = std::str::from_utf8(sequence)?;
            let expected_size = self.sequence_size;
            let observed_size = sequence.len();
            bail!(
                "Probe sequence size mismatch\nExpected size: {expected_size}\nFound size: {observed_size}\nSequence: {sequence_str}"
            )
        }
    }

    /// Insert a sequence-alias pairing into the map
    pub fn insert(&mut self, sequence: &Sequence, index: usize) -> Result<()> {
        self.update_sequence_size(sequence)?;
        self.map.insert(sequence.clone(), index);
        self.seq_map.insert(index, sequence.clone());

        let (left_seq, right_seq) = sequence.split_at(self.left_size);
        self.left_map.insert(left_seq.to_owned(), index);
        self.right_map.insert(right_seq.to_owned(), index);

        self.left_correction.insert(left_seq);
        self.right_correction.insert(right_seq);

        Ok(())
    }

    /// Determine the target index for a given sequence following the `CellRanger` Flex protocol
    ///
    /// 1. if both left and right indices match to the same index, return the index
    /// 2. if both left and right indices match to differenct indices, return None
    /// 3. if both left and right indices are None, return None
    /// 4. if only one index is Some, perform a hamming distance check and return the index if the distance is within the threshold
    fn determine_target(
        &self,
        sequence: SeqRef,
        left_seq: SeqRef,
        right_seq: SeqRef,
    ) -> Option<usize> {
        let left_idx = self.left_map.get(left_seq).copied();
        let right_idx = self.right_map.get(right_seq).copied();
        match (left_idx, right_idx) {
            (Some(left), Some(right)) => {
                if left == right {
                    Some(left)
                } else {
                    None
                }
            }
            (None, None) => None,
            (Some(t), None) | (None, Some(t)) => {
                let expected_seq = self.seq_map.get(&t).unwrap();
                if within_hdist(sequence, expected_seq, MAXIMUM_DISTANCE) {
                    Some(t)
                } else {
                    None
                }
            }
        }
    }

    pub fn match_sequence(&self, sequence: SeqRef) -> Option<usize> {
        let left_seq = &sequence[..self.left_size];
        let right_seq = &sequence[self.left_size..];
        self.determine_target(sequence, left_seq, right_seq)
    }

    pub fn match_corrected_sequence(&self, sequence: SeqRef) -> Option<usize> {
        let left_seq = &sequence[..self.left_size];
        let right_seq = &sequence[self.left_size..];

        let left_parent = self.left_correction.get_parent(left_seq);
        let right_parent = self.right_correction.get_parent(right_seq);

        match (left_parent, right_parent) {
            (Some(left_parent), Some(right_parent)) => {
                self.determine_target(sequence, &left_parent.0, &right_parent.0)
            }
            (Some(left_parent), None) => self.determine_target(sequence, &left_parent.0, right_seq),
            (None, Some(right_parent)) => {
                self.determine_target(sequence, left_seq, &right_parent.0)
            }
            (None, None) => self.determine_target(sequence, left_seq, right_seq),
        }
    }

    /// Get the length of the map
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[derive(Debug, Clone)]
pub struct MapIndexToName {
    map: Vec<Name>,
}
impl MapIndexToName {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: Vec::with_capacity(capacity),
        }
    }

    /// Insert an index-alias pairing into the map
    pub fn insert(&mut self, index: usize, name: Name) {
        self.map.insert(index, name);
    }

    /// Get an alias by index
    pub fn get(&self, index: usize) -> Option<&Name> {
        self.map.as_slice().get(index)
    }

    /// Get an iterator over the map
    pub fn iter_records(&self) -> impl Iterator<Item = &str> {
        self.map
            .iter()
            .map(|name| std::str::from_utf8(name).unwrap())
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn within_hdist(u: SeqRef, v: SeqRef, max_dist: usize) -> bool {
    let mut dist = 0;
    for (a, b) in u.iter().zip(v.iter()) {
        if a != b {
            dist += 1;
        }
        if dist > max_dist {
            return false;
        }
    }
    true
}
