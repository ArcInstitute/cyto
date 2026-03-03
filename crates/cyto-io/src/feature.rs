use serde::Serialize;

/// Trait for writing library features to a file in a specific format.
pub trait FeatureWriter<'a> {
    type Record: Serialize;

    /// Iterator over the records to be written.
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record>;

    /// Writes the record stream to a given writer
    fn write_to<W: std::io::Write>(&'a self, writer: W) -> std::io::Result<()> {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_writer(writer);
        for record in self.record_stream() {
            wtr.serialize(record)?;
        }
        wtr.flush()?;
        Ok(())
    }
}
