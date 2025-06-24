use anyhow::{bail, Result};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
pub struct BarcodeIndexCount {
    barcode: u64,
    index: u64,
    count: u64,
}
impl BarcodeIndexCount {
    pub fn barcode(&self) -> u64 {
        self.barcode
    }
    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn count(&self) -> u64 {
        self.count
    }
}

#[derive(Debug, Default)]
pub struct BarcodeIndexCounts {
    /// Indexed by `barcode` then `index` then `counts`
    inner: HashMap<u64, HashMap<u64, u64>>,
}
impl BarcodeIndexCounts {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, barcode: u64, index: u64) {
        self.insert_count(barcode, index, 1);
    }

    pub fn insert_count(&mut self, barcode: u64, index: u64, count: u64) {
        let barcode_map = self.inner.entry(barcode).or_default();
        let inner_count = barcode_map.entry(index).or_insert(0);
        *inner_count += count
    }

    pub fn iter_counts(&self) -> impl Iterator<Item = BarcodeIndexCount> + '_ {
        self.inner.iter().flat_map(|(barcode, index_counts)| {
            index_counts
                .iter()
                .map(|(index, counts)| BarcodeIndexCount {
                    barcode: *barcode,
                    index: *index,
                    count: *counts,
                })
        })
    }

    /// Used for testing purposes
    #[allow(dead_code)]
    fn get_abundance(&self, barcode: u64, index: u64) -> Option<u64> {
        self.inner
            .get(&barcode)
            .and_then(|index_counts| index_counts.get(&index).copied())
    }

    /// Used for testing purposes
    #[allow(dead_code)]
    fn get_num_indices(&self, barcode: u64) -> Option<usize> {
        self.inner.get(&barcode).map(HashMap::len)
    }

    /// Get the total number of barcodes in the map
    pub fn get_num_barcodes(&self) -> usize {
        self.inner.len()
    }
}

#[derive(Debug, Default)]
pub struct UmiState {
    max_index: u64,
    max_abundance: u64,
    current_index: u64,
    current_abundance: u64,
}
impl UmiState {
    /// Increment the current abundance
    pub fn update_max(&mut self) {
        if self.current_abundance > self.max_abundance {
            self.max_index = self.current_index;
            self.max_abundance = self.current_abundance;
        }
    }

    /// Update the current index
    pub fn update_index(&mut self, new_index: u64) {
        self.current_index = new_index;
        self.current_abundance = 1;
    }

    /// Reset to a new index
    pub fn reset(&mut self, new_index: u64) {
        self.current_index = new_index;
        self.current_abundance = 1;

        self.max_index = new_index;
        self.max_abundance = 1;
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum DeduplicateError {
    #[error("IBU is not sorted. Please sort the IBU before counting.")]
    UnsortedIbu,
    #[error("Index value ({}) exceeds the maximum index value provided ({}). Likely an incorrect feature list.", .index, .max_index)]
    MaxIndexExceeded { index: u64, max_index: u64 },
    #[error("No records found in the input stream.")]
    EmptyStream,
}

pub fn deduplicate_umis(
    mut record_stream: impl Iterator<Item = Result<ibu::Record, ibu::BinaryFormatError>>,
    max_index: u64,
) -> Result<BarcodeIndexCounts> {
    let mut counts = BarcodeIndexCounts::new();
    let mut umi_state = UmiState::default();

    let mut last_record = if let Some(record) = record_stream.next() {
        record?
    } else {
        bail!(DeduplicateError::EmptyStream);
    };

    for record in record_stream {
        let record = record?;

        if record.index() > max_index {
            bail!(DeduplicateError::MaxIndexExceeded {
                index: record.index(),
                max_index
            });
        }

        // Designates an unsorted IBU
        if last_record > record {
            bail!(DeduplicateError::UnsortedIbu);
        }

        // Handle new barcode
        if last_record.barcode() != record.barcode() {
            // Process the last UMI group before moving to new barcode
            umi_state.update_max();
            counts.insert(last_record.barcode(), umi_state.max_index);
            umi_state.reset(record.index());
        } else if last_record.umi() != record.umi() {
            // Process the current UMI group before moving to new UMI
            umi_state.update_max();
            counts.insert(last_record.barcode(), umi_state.max_index);
            umi_state.reset(record.index());
        } else if last_record.index() != record.index() {
            // Handle new index within same UMI
            umi_state.update_max();
            umi_state.update_index(record.index());
        } else {
            // Same index within same UMI
            umi_state.current_abundance += 1;
        }

        // Update the last record
        last_record = record;
    }

    // Process the final record
    umi_state.update_max();
    counts.insert(last_record.barcode(), umi_state.max_index);

    Ok(counts)
}

#[cfg(test)]
mod testing {
    use anyhow::Result;

    pub fn build_record_stream(
        records: Vec<ibu::Record>,
    ) -> impl Iterator<Item = Result<ibu::Record, ibu::BinaryFormatError>> {
        records.into_iter().map(Ok)
    }

    #[test]
    fn test_skip_redudant_records() -> Result<()> {
        let records = vec![ibu::Record::new(1, 1, 0); 100];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(1));
        assert_eq!(counts.get_abundance(1, 0), Some(1));
        Ok(())
    }

    #[test]
    fn test_single_barcode_dedup() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(1));
        assert_eq!(counts.get_abundance(1, 0), Some(2));

        Ok(())
    }

    #[test]
    fn test_single_barcode_dedup_index_tie() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0), // precedence goes to first observed in tie
            ibu::Record::new(1, 2, 1),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(1));
        assert_eq!(counts.get_abundance(1, 0), Some(2));

        Ok(())
    }

    #[test]
    fn test_single_barcode_dedup_multiple_index_order_first() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0),
            ibu::Record::new(1, 2, 0),
            ibu::Record::new(1, 2, 0),
            ibu::Record::new(1, 2, 0), // clear winner with 4
            ibu::Record::new(1, 2, 1),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(1));
        assert_eq!(counts.get_abundance(1, 0), Some(2));

        Ok(())
    }

    #[test]
    fn test_single_barcode_dedup_multiple_index_order_second() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0), // likely an error since it's only observed once
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1), // clear winner with 4
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(2));
        assert_eq!(counts.get_abundance(1, 0), Some(1));
        assert_eq!(counts.get_abundance(1, 1), Some(1));

        Ok(())
    }

    #[test]
    fn test_single_barcode_dedup_multiple_index_order_second_with_new_umi_same_as_previous(
    ) -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0), // likely an error since it's only observed once
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1), // clear winner with 4
            ibu::Record::new(1, 3, 1), // new umi with same index as previous
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(2));
        assert_eq!(counts.get_abundance(1, 0), Some(1));
        assert_eq!(counts.get_abundance(1, 1), Some(2));

        Ok(())
    }

    #[test]
    fn test_single_barcode_dedup_multiple_index_order_second_with_new_umi_diff_to_previous(
    ) -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0), // likely an error since it's only observed once
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1), // clear winner with 4
            ibu::Record::new(1, 3, 2), // new umi with different index to previous
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 1);
        assert_eq!(counts.get_num_indices(1), Some(3));
        assert_eq!(counts.get_abundance(1, 0), Some(1));
        assert_eq!(counts.get_abundance(1, 1), Some(1));
        assert_eq!(counts.get_abundance(1, 2), Some(1));

        Ok(())
    }

    #[test]
    fn test_multiple_barcodes_same_umi_index() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(2, 1, 0),
            ibu::Record::new(3, 1, 0),
            ibu::Record::new(4, 1, 0),
            ibu::Record::new(5, 1, 0),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 5);
        for i in 1..6 {
            assert_eq!(counts.get_num_indices(i), Some(1));
            assert_eq!(counts.get_abundance(i, 0), Some(1));
        }

        Ok(())
    }

    #[test]
    fn test_multiple_barcodes_same_duplicate_umi_index() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(2, 1, 0),
            ibu::Record::new(2, 1, 0),
            ibu::Record::new(3, 1, 0),
            ibu::Record::new(3, 1, 0),
            ibu::Record::new(4, 1, 0),
            ibu::Record::new(4, 1, 0),
            ibu::Record::new(5, 1, 0),
            ibu::Record::new(5, 1, 0),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 5);
        for i in 1..6 {
            assert_eq!(counts.get_num_indices(i), Some(1));
            assert_eq!(counts.get_abundance(i, 0), Some(1));
        }

        Ok(())
    }

    #[test]
    fn test_multiple_barcodes_same_multiple_umi_index() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 0),
            ibu::Record::new(2, 1, 0),
            ibu::Record::new(2, 2, 0),
            ibu::Record::new(3, 1, 0),
            ibu::Record::new(3, 2, 0),
            ibu::Record::new(4, 1, 0),
            ibu::Record::new(4, 2, 0),
            ibu::Record::new(5, 1, 0),
            ibu::Record::new(5, 2, 0),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 5);
        for i in 1..6 {
            assert_eq!(counts.get_num_indices(i), Some(1));
            assert_eq!(counts.get_abundance(i, 0), Some(2));
        }

        Ok(())
    }

    #[test]
    fn test_multiple_barcodes_same_multiple_umi_multiple_index() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(2, 1, 0),
            ibu::Record::new(2, 2, 1),
            ibu::Record::new(3, 1, 0),
            ibu::Record::new(3, 2, 1),
            ibu::Record::new(4, 1, 0),
            ibu::Record::new(4, 2, 1),
            ibu::Record::new(5, 1, 0),
            ibu::Record::new(5, 2, 1),
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 5);
        for i in 1..6 {
            assert_eq!(counts.get_num_indices(i), Some(2));
            assert_eq!(counts.get_abundance(i, 0), Some(1));
            assert_eq!(counts.get_abundance(i, 1), Some(1));
        }

        Ok(())
    }

    #[test]
    fn test_multiple_barcodes_same_multiple_umi_multiple_index_max_condition() -> Result<()> {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 2, 2), // likely an error since it's only observed once
            ibu::Record::new(2, 1, 0),
            ibu::Record::new(2, 2, 1),
            ibu::Record::new(2, 2, 1),
            ibu::Record::new(2, 2, 2), // likely an error since it's only observed once
            ibu::Record::new(3, 1, 0),
            ibu::Record::new(3, 2, 1),
            ibu::Record::new(3, 2, 1),
            ibu::Record::new(3, 2, 2), // likely an error since it's only observed once
            ibu::Record::new(4, 1, 0),
            ibu::Record::new(4, 2, 1),
            ibu::Record::new(4, 2, 1),
            ibu::Record::new(4, 2, 2), // likely an error since it's only observed once
            ibu::Record::new(5, 1, 0),
            ibu::Record::new(5, 2, 1),
            ibu::Record::new(5, 2, 1),
            ibu::Record::new(5, 2, 2), // likely an error since it's only observed once
        ];
        let stream = build_record_stream(records);
        let counts = super::deduplicate_umis(stream, u64::MAX)?;

        assert_eq!(counts.get_num_barcodes(), 5);
        for i in 1..6 {
            assert_eq!(counts.get_num_indices(i), Some(2));
            assert_eq!(counts.get_abundance(i, 0), Some(1));
            assert_eq!(counts.get_abundance(i, 1), Some(1));
        }

        Ok(())
    }

    #[test]
    fn test_incorrect_feature_size() {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 1),
            ibu::Record::new(1, 1, 2),
            ibu::Record::new(1, 1, 3),
            ibu::Record::new(1, 1, 4),
            ibu::Record::new(1, 1, 5),
        ];
        let stream = build_record_stream(records);
        let result = super::deduplicate_umis(stream, 3);
        assert_eq!(
            result
                .unwrap_err()
                .downcast::<super::DeduplicateError>()
                .unwrap(),
            super::DeduplicateError::MaxIndexExceeded {
                index: 4,
                max_index: 3
            },
        );
    }

    #[test]
    fn test_unsorted_input_barcode() {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(2, 1, 1),
            ibu::Record::new(1, 1, 2),
        ];
        let stream = build_record_stream(records);
        let result = super::deduplicate_umis(stream, u64::MAX);
        assert_eq!(
            result
                .unwrap_err()
                .downcast::<super::DeduplicateError>()
                .unwrap(),
            super::DeduplicateError::UnsortedIbu,
        );
    }

    #[test]
    fn test_unsorted_input_umi() {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 2, 1),
            ibu::Record::new(1, 1, 2),
        ];
        let stream = build_record_stream(records);
        let result = super::deduplicate_umis(stream, u64::MAX);
        assert_eq!(
            result
                .unwrap_err()
                .downcast::<super::DeduplicateError>()
                .unwrap(),
            super::DeduplicateError::UnsortedIbu,
        );
    }

    #[test]
    fn test_unsorted_input_index() {
        let records = vec![
            ibu::Record::new(1, 1, 0),
            ibu::Record::new(1, 1, 1),
            ibu::Record::new(1, 1, 0),
        ];
        let stream = build_record_stream(records);
        let result = super::deduplicate_umis(stream, u64::MAX);
        assert_eq!(
            result
                .unwrap_err()
                .downcast::<super::DeduplicateError>()
                .unwrap(),
            super::DeduplicateError::UnsortedIbu,
        );
    }

    #[test]
    fn test_empty_stream() {
        let records = vec![];
        let stream = build_record_stream(records);
        let result = super::deduplicate_umis(stream, u64::MAX);
        assert_eq!(
            result
                .unwrap_err()
                .downcast::<super::DeduplicateError>()
                .unwrap(),
            super::DeduplicateError::EmptyStream,
        );
    }
}
