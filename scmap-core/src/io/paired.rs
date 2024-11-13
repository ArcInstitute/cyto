use anyhow::Result;
use fxread::{initialize_reader, FastxRead, Record};

use crate::Bus;

pub struct PairedRecord {
    pub r1: Record,
    pub r2: Record,
}
impl PairedRecord {
    pub fn as_bus(&self, size_barcode: usize, size_umi: usize) -> Bus {
        Bus::new(self.r1.seq(), self.r2.seq(), size_barcode, size_umi)
    }
}

pub struct PairedReader {
    reader_r1: Box<dyn FastxRead<Item = Record>>,
    reader_r2: Box<dyn FastxRead<Item = Record>>,
}
impl PairedReader {
    pub fn new(filepath_r1: &str, filepath_r2: &str) -> Result<Self> {
        let reader_r1 = initialize_reader(filepath_r1)?;
        let reader_r2 = initialize_reader(filepath_r2)?;
        Ok(Self {
            reader_r1,
            reader_r2,
        })
    }
}
impl Iterator for PairedReader {
    type Item = PairedRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let r1 = self.reader_r1.next()?;
        let r2 = self.reader_r2.next()?;
        Some(PairedRecord { r1, r2 })
    }
}
