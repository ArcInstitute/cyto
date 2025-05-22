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
    with_header: bool,
}
impl<'a> BarcodeIndexWriter<'a> {
    pub fn new(inner: &'a BarcodeIndexCounter, with_header: bool) -> Self {
        Self { inner, with_header }
    }

    pub fn write_sparse<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut writer = csv::WriterBuilder::default()
            .delimiter(b'\t')
            .has_headers(self.with_header)
            .from_writer(writer);

        self.inner
            .iter_records()
            .try_for_each(|record| -> Result<(), csv::Error> { writer.serialize(record) })?;

        writer.flush()?;
        Ok(())
    }
}
