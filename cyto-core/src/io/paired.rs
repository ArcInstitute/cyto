use anyhow::Result;
use seq_io::fastq::{Reader, Record, RefRecord};
use std::io::{Read, Write};

use crate::Bus;

pub struct PairedRecord<'a> {
    pub r1: RefRecord<'a>,
    pub r2: RefRecord<'a>,
}
impl<'a> PairedRecord<'a> {
    pub fn new(r1: RefRecord<'a>, r2: RefRecord<'a>) -> Self {
        Self { r1, r2 }
    }
    pub fn as_bus(&self, size_barcode: usize, size_umi: usize) -> Result<Bus> {
        Bus::new(self.r1.seq(), self.r2.seq(), size_barcode, size_umi)
    }
}

pub struct PairedReader {
    r1_reader: Reader<Box<dyn Read>>,
    r2_reader: Reader<Box<dyn Read>>,
}
impl PairedReader {
    pub fn new(r1_filepath: &str, r2_filepath: &str) -> Result<Self> {
        let (r1_handle, _) = niffler::from_path(r1_filepath)?;
        let (r2_handle, _) = niffler::from_path(r2_filepath)?;

        let r1_reader = Reader::new(r1_handle);
        let r2_reader = Reader::new(r2_handle);

        Ok(Self {
            r1_reader,
            r2_reader,
        })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<PairedRecord>> {
        let r1 = self.r1_reader.next();
        let r2 = self.r2_reader.next();

        match (r1, r2) {
            (Some(r1), Some(r2)) => match (r1, r2) {
                (Ok(r1), Ok(r2)) => {
                    let pair = PairedRecord::new(r1, r2);
                    Some(Ok(pair))
                }
                (Err(e), _) | (_, Err(e)) => Some(Err(e.into())),
            },
            (Some(_), None) => Some(Err(anyhow::anyhow!("Unexpected end of R2 file"))),
            (None, Some(_)) => Some(Err(anyhow::anyhow!("Unexpected end of R1 file"))),
            (None, None) => None,
        }
    }

    pub fn write_to<W: Write>(&mut self, writer: W, barcode: usize, umi: usize) -> Result<()> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_writer(writer);

        while let Some(pair) = self.next() {
            let pair = pair?;
            let Ok(bus) = pair.as_bus(barcode, umi) else {
                continue;
            };
            let record = (
                &bus.str_barcode()?,
                &bus.str_umi()?,
                std::str::from_utf8(bus.seq)?,
            );
            wtr.serialize(record)?;
        }
        wtr.flush()?;

        Ok(())
    }

    pub fn append_to<W: Write>(
        &mut self,
        writer: &mut W,
        barcode: usize,
        umi: usize,
    ) -> Result<()> {
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_writer(writer);

        while let Some(pair) = self.next() {
            let pair = pair?;
            let Ok(bus) = pair.as_bus(barcode, umi) else {
                continue;
            };
            let record = (
                &bus.str_barcode()?,
                &bus.str_umi()?,
                std::str::from_utf8(bus.seq)?,
            );
            wtr.serialize(record)?;
        }
        wtr.flush()?;

        Ok(())
    }
}
