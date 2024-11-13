use crate::Bus;
use hashbrown::{HashMap, HashSet};

type Barcode = Vec<u8>;
type Umi = Vec<u8>;
type Index = usize;

type UmiSet = HashSet<Umi>;
type IndexSet = HashMap<Index, UmiSet>;
type BarcodeSet = HashMap<Barcode, IndexSet>;

#[derive(Default, Debug)]
pub struct BusCounter {
    map: BarcodeSet,
    max_index: Index,
}
impl BusCounter {
    /// Increments the counter for the given Bus and index
    pub fn increment(&mut self, bus: &Bus, index: Index) {
        self.ensure_barcode_exists(bus.barcode);
        self.ensure_index_exists(bus.barcode, index);
        self.add_umi(bus.barcode, index, bus.umi);
        self.update_max_index(index);
    }

    /// Ensures that the barcode exists in the map
    fn ensure_barcode_exists(&mut self, barcode: &[u8]) {
        if !self.map.contains_key(barcode) {
            self.map.insert(barcode.to_vec(), HashMap::new());
        }
    }

    /// Ensures that the index exists for the given barcode
    fn ensure_index_exists(&mut self, barcode: &[u8], index: Index) {
        let index_set = self.map.get_mut(barcode).unwrap();
        if !index_set.contains_key(&index) {
            index_set.insert(index, HashSet::new());
        }
    }

    /// Adds a UMI to the set for the given barcode and index
    fn add_umi(&mut self, barcode: &[u8], index: Index, umi: &[u8]) {
        let index_set = self.map.get_mut(barcode).unwrap();
        let umi_set = index_set.get_mut(&index).unwrap();
        if !umi_set.contains(umi) {
            umi_set.insert(umi.to_vec());
        }
    }

    /// Updates the maximum index seen so far
    /// This is used to determine the number of columns in the output matrix
    fn update_max_index(&mut self, index: Index) {
        self.max_index = self.max_index.max(index);
    }

    /// Gets the number of UMIs for a given barcode and index
    /// Returns Some(0) if the index does not exist
    /// Returns None if the barcode does not exist
    pub fn get_index_abundance(&self, barcode: &[u8], index: Index) -> Option<usize> {
        if let Some(index_set) = self.map.get(barcode) {
            index_set.get(&index).map(|set| set.len()).or(Some(0))
        } else {
            None
        }
    }

    /// Gets the number of barcodes in the map
    pub fn num_barcodes(&self) -> usize {
        self.map.len()
    }

    /// Iterates over the barcodes in the map
    pub fn iter_barcodes(&self) -> impl Iterator<Item = &[u8]> {
        self.map.keys().map(|k| k.as_slice())
    }

    pub fn max_index(&self) -> Index {
        self.max_index
    }
}
