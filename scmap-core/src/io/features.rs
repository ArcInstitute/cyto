use serde::Serialize;

pub trait FeatureWriter<'a> {
    type Record: Serialize;
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record>;
    fn write_to<W: std::io::Write>(&'a self, writer: W) -> std::io::Result<()> {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(writer);
        for record in self.record_stream() {
            wtr.serialize(record)?;
        }
        wtr.flush()?;
        Ok(())
    }
}
