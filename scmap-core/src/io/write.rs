use anyhow::Result;
use std::io::Write;

use crate::BarcodeIndexCounter;

pub fn write_sparse_mtx<W: Write>(
    writer: &mut W,
    counter: &BarcodeIndexCounter,
    with_header: bool,
) -> Result<()> {
    let bcw = BarcodeIndexWriter::new(counter, with_header);
    bcw.write_sparse(writer)
}

pub struct BarcodeIndexWriter<'a> {
    inner: &'a BarcodeIndexCounter,
    num_indices: usize,
    with_header: bool,
}
impl<'a> BarcodeIndexWriter<'a> {
    pub fn new(inner: &'a BarcodeIndexCounter, with_header: bool) -> Self {
        let num_indices = inner.max_index();
        Self {
            inner,
            num_indices,
            with_header,
        }
    }

    pub fn write_sparse<W: Write>(&self, writer: &mut W) -> Result<()> {
        if self.with_header {
            self.write_sparse_header(writer)?;
        }
        self.write_rows_sparse(writer)
    }

    fn write_sparse_header<W: Write>(&self, writer: &mut W) -> Result<()> {
        writeln!(writer, "barcode\tindex\tabundance")?;
        Ok(())
    }

    fn write_rows_sparse<W: Write>(&self, writer: &mut W) -> Result<()> {
        for barcode in self.inner.iter_barcodes() {
            for index in 0..self.num_indices {
                if let Some(abundance) = self.inner.get_index_abundance(barcode, index) {
                    if abundance > 0 {
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
        }
        Ok(())
    }
}
