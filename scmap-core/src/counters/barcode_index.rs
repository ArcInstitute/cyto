use hashbrown::HashMap;

use super::{Barcode, BusCounter, Index, IndexCounts};

/// A data structure that stores the counts of each index for each barcode
/// after accounting for UMI deduplication
#[derive(Debug, Default)]
pub struct BarcodeIndexCounter {
    // The abundances of each index for each barcode after UMI deduplication
    map: HashMap<Barcode, IndexCounts>,

    // The maximum index seen in the data after UMI deduplication
    max_index: Index,
}
impl BarcodeIndexCounter {
    pub fn from_counter(counter: &BusCounter) -> Self {
        let mut map = HashMap::default();
        let mut max_index = 0;

        for barcode in counter.iter_barcodes() {
            // Ensures that the barcode exists in the map
            if !map.contains_key(barcode) {
                map.insert(barcode.to_vec(), IndexCounts::default());
            }
            // Pulls the barcode from the map
            let barcode_map = map.get_mut(barcode).unwrap();

            // Pulls the UMI set for the barcode from the counter
            let umi_set = counter.get_umi_set(barcode).unwrap();

            // Iterates over the UMIs for that barcode
            for umi in umi_set.keys() {
                // Pulls the tracked collection of Indices for the given Barcode-UMI pair
                let tracked_index = umi_set.get(umi).unwrap();

                // Increments a single count for the top index of that Barcode-UMI pair
                *barcode_map.entry(tracked_index.top_index()).or_default() += 1;

                // Updates the max index if necessary
                if tracked_index.max_index > max_index {
                    max_index = tracked_index.max_index;
                }
            }
        }

        Self { map, max_index }
    }

    pub fn max_index(&self) -> Index {
        self.max_index
    }

    pub fn iter_barcodes(&self) -> impl Iterator<Item = &Barcode> {
        self.map.keys()
    }

    pub fn get_index_abundance(&self, barcode: &[u8], index: Index) -> Option<usize> {
        if let Some(indices) = self.map.get(barcode) {
            if let Some(abundance) = indices.get(&index) {
                Some(*abundance)
            } else {
                Some(0)
            }
        } else {
            None
        }
    }
}
