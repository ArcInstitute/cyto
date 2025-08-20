use std::sync::Arc;

use anyhow::{Result, bail};
use parking_lot::Mutex;

pub struct BarcodeSetReader<It>
where
    It: Iterator<Item = Result<ibu::Record, ibu::BinaryFormatError>>,
{
    reader: It,
    remainder: Option<ibu::Record>,
}
impl<It> BarcodeSetReader<It>
where
    It: Iterator<Item = Result<ibu::Record, ibu::BinaryFormatError>>,
{
    pub fn new(reader: It) -> Self {
        BarcodeSetReader {
            reader,
            remainder: None,
        }
    }

    pub fn new_shared(reader: It) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new(reader)))
    }

    /// Fills a vector with records with the same barcode.
    ///
    /// returns true if the vector is not empty, false if the reader is exhausted
    pub fn fill_barcode_set(&mut self, bset: &mut Vec<ibu::Record>) -> Result<bool> {
        let mut last_record = None;
        if let Some(record) = self.remainder.take() {
            bset.push(record);
            last_record = Some(record);
        }
        for record in self.reader.by_ref() {
            let record = record?;
            if let Some(last) = last_record {
                if record < last {
                    bail!("Input is unsorted; expecting sorted IBU input for UMI correction");
                }
                if record.barcode() == last.barcode() {
                    bset.push(record);
                } else {
                    self.remainder = Some(record);
                    break;
                }
            } else {
                bset.push(record);
                last_record = Some(record);
            }
        }
        Ok(!bset.is_empty())
    }
}
