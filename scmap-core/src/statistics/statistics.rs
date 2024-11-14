use serde::Serialize;

use super::{LibraryCombination, MappingStatistics, ProcessedMappingStatistics};

#[derive(Debug, Serialize, Clone)]
pub struct Statistics {
    library: LibraryCombination,
    mapping: ProcessedMappingStatistics,
}
impl Statistics {
    pub fn new(library: LibraryCombination, mapping: MappingStatistics) -> Self {
        Self {
            library,
            mapping: mapping.into(),
        }
    }
    pub fn save_json<W: std::io::Write>(&self, writer: W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}
