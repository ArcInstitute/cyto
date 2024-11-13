use std::io::Write;

use crate::BusCounter;
use anyhow::Result;

pub struct BusCounterWriter<'a> {
    inner: &'a BusCounter,
    num_indices: usize,
}
impl<'a> BusCounterWriter<'a> {
    pub fn new(inner: &'a BusCounter) -> Self {
        let num_indices = inner.max_index();
        Self { inner, num_indices }
    }

    /// Writes a sparse MTX file to the given writer
    pub fn write_sparse<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.write_sparse_header(writer)?;
        self.write_rows_sparse(writer)?;
        Ok(())
    }

    fn write_sparse_header<W: Write>(&self, writer: &mut W) -> Result<()> {
        writeln!(writer, "barcode\tindex\tabundance")?;
        Ok(())
    }

    fn write_rows_sparse<W: Write>(&self, writer: &mut W) -> Result<()> {
        for barcode in self.inner.iter_barcodes() {
            for index in 0..self.num_indices {
                if let Some(abundance) = self.inner.get_index_abundance(barcode, index) {
                    if abundance == 0 {
                        continue;
                    }
                    writeln!(
                        writer,
                        "{}\t{}\t{}",
                        std::str::from_utf8(barcode)?,
                        index,
                        abundance
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Writes a 2D matrix to a writer.
    ///
    /// Barcodes are written in rows, and indices are written in columns.
    /// Each cell contains the abundance of the barcode-index pair.
    pub fn write_matrix<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.write_header(writer)?;
        self.write_rows(writer)?;
        Ok(())
    }

    fn write_header<W: Write>(&self, writer: &mut W) -> Result<()> {
        write!(writer, "barcode")?;
        for index in 0..self.num_indices {
            write!(writer, "\t{}", index)?;
        }
        writeln!(writer)?;
        Ok(())
    }

    fn write_rows<W: Write>(&self, writer: &mut W) -> Result<()> {
        for barcode in self.inner.iter_barcodes() {
            write!(writer, "{}", std::str::from_utf8(barcode)?)?;
            for index in 0..self.num_indices {
                let abundance = self.inner.get_index_abundance(barcode, index).unwrap_or(0);
                write!(writer, "\t{}", abundance)?;
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}
