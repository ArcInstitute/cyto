use super::{BarcodeIndexCounter, BarcodeSet, Counter, Index, TrackedIndexCounter, UmiSet};
use crate::{aliases::SeqRef, Bus};

/// `BusCounter` is a data structure that manages the counts of UMIs for each barcode and index
///
/// It handles UMI deduplication and keeps track of the maximum index seen to determine the number of columns in the output matrix
///
/// Internally it can be thought of as a sparse 3 dimensional matrix where the dimensions are:
/// 1. Barcode
/// 2. Index
/// 3. UMI
/// 4. Count (the value of each coordinate is the number of times that combination of barcode, index, and UMI was seen)
///
/// UMI deduplication is done by returning the maximum index for each Barcode-Umi pair.
#[derive(Default, Debug)]
pub struct BusCounter {
    map: BarcodeSet,
    max_index: Index,
}
impl BusCounter {
    /// Ensures that the barcode exists in the map by inserting it if it does not
    fn ensure_barcode_exists(&mut self, barcode: SeqRef) {
        if !self.map.contains_key(barcode) {
            self.map.insert(barcode.to_vec(), UmiSet::default());
        }
    }

    /// Ensures that the UMI exists for the given barcode by inserting it if it does not
    fn ensure_umi_exists(&mut self, barcode: SeqRef, umi: SeqRef) {
        let umi_set = self.map.get_mut(barcode).unwrap();
        if !umi_set.contains_key(umi) {
            umi_set.insert(umi.to_vec(), TrackedIndexCounter::default());
        }
    }

    /// Increments the `Index` count for the given `Barcode` and `Umi`
    fn increment_index(&mut self, barcode: SeqRef, umi: SeqRef, index: Index) {
        // Handles path initialization and validation within the tree
        self.ensure_barcode_exists(barcode);
        self.ensure_umi_exists(barcode, umi);

        // Selects the necessary node within the tree
        let umi_set = self.map.get_mut(barcode).unwrap();
        let index_counts = umi_set.get_mut(umi).unwrap();
        index_counts.increment(index);
    }

    /// Updates the maximum index seen so far
    /// This is used to determine the number of columns in the output matrix
    fn update_max_index(&mut self, index: Index) {
        self.max_index = self.max_index.max(index);
    }

    /// Returns the UMI set for a given barcode
    pub fn get_umi_set(&self, barcode: SeqRef) -> Option<&UmiSet> {
        self.map.get(barcode)
    }

    /// Gets the number of barcodes in the map
    pub fn num_barcodes(&self) -> usize {
        self.map.len()
    }

    /// Iterates over the barcodes in the map
    pub fn iter_barcodes(&self) -> impl Iterator<Item = SeqRef> {
        self.map.keys().map(std::vec::Vec::as_slice)
    }

    pub fn max_index(&self) -> Index {
        self.max_index
    }

    pub fn dedup_umi(&self) -> BarcodeIndexCounter {
        BarcodeIndexCounter::from_counter(self)
    }
}

impl Counter for BusCounter {
    fn increment(&mut self, bus: &Bus, index: Index) {
        self.increment_index(bus.barcode, bus.umi, index);
        self.update_max_index(index);
    }
}
