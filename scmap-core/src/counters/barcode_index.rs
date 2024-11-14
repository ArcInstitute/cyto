use serde::Serialize;

use super::{Barcode, BusCounter, Index, IndexCounts};
use crate::io::utils::bytes_to_string;

#[derive(Debug, Clone, Serialize)]
pub struct BarcodeIndex {
    #[serde(serialize_with = "bytes_to_string")]
    barcode: Barcode,
    index: Index,
    abundance: usize,
}

/// A data structure that stores the counts of each index for each barcode
/// after accounting for UMI deduplication
#[derive(Debug, Default)]
pub struct BarcodeIndexCounter {
    inner: Vec<BarcodeIndex>,
}
impl BarcodeIndexCounter {
    pub fn from_counter(counter: &BusCounter) -> Self {
        let mut inner = Vec::default();
        let mut max_index = 0;

        for barcode in counter.iter_barcodes() {
            // Pulls the UMI set for the barcode from the counter
            let umi_set = counter.get_umi_set(barcode).unwrap();

            // Iterates over the UMIs for that barcode
            let mut index_counts = IndexCounts::default();
            for umi in umi_set.keys() {
                // Pulls the tracked collection of Indices for the given Barcode-UMI pair
                let tracked_index = umi_set.get(umi).unwrap();

                // Increments a single count for the top index of that Barcode-UMI pair
                *index_counts.entry(tracked_index.top_index()).or_default() += 1;

                // Updates the max index if necessary
                if tracked_index.max_index > max_index {
                    max_index = tracked_index.max_index;
                }
            }

            // Iterates over the index counts and creates a BarcodeIndex for each
            for (index, abundance) in &index_counts {
                inner.push(BarcodeIndex {
                    barcode: barcode.to_vec(), // TODO: This is a clone - could be optimized
                    index: *index,
                    abundance: *abundance,
                });
            }
        }

        Self { inner }
    }

    pub fn iter_records(&self) -> impl Iterator<Item = &BarcodeIndex> {
        self.inner.iter()
    }
}
