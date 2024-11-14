use serde::Serialize;

pub trait FeatureWriter<'a> {
    type Record: Serialize;
    fn record_stream(&'a self) -> impl Iterator<Item = Self::Record>;
}
