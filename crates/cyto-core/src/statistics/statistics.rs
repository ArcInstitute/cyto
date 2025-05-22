use serde::Serialize;

use super::{LibraryCombination, MappingStatistics, ProcessedMappingStatistics, RuntimeStatistics};

#[derive(Debug, Serialize, Clone)]
pub struct Statistics {
    library: LibraryCombination,
    mapping: ProcessedMappingStatistics,
    runtime: RuntimeStatistics,
}
impl Statistics {
    pub fn new(
        library: LibraryCombination,
        mapping: MappingStatistics,
        runtime: RuntimeStatistics,
    ) -> Self {
        Self {
            library,
            mapping: mapping.into(),
            runtime,
        }
    }
    pub fn save_json<W: std::io::Write>(&self, writer: W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}
