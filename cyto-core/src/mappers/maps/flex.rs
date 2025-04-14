use anyhow::{bail, Result};
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
        Ok(())
    }

    pub fn match_sequence(&self, sequence: SeqRef) -> Option<usize> {
        let left_idx = self.left_map.get(&sequence[..self.left_size]).copied();
        let right_idx = self.right_map.get(&sequence[self.left_size..]).copied();
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
                eprintln!("Running alignment");
                let expected_seq = self.seq_map.get(&t).unwrap();
                if within_hdist(sequence, expected_seq, MAXIMUM_DISTANCE) {
                    Some(t)
                } else {
                    None
                }
            }
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
