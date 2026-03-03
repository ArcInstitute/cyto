mod geometry;
mod mapper;
mod processor;
mod run;
mod stats;
mod utils;

const GEX_MAX_HDIST: usize = 3;
type BoxedWriter = Box<dyn std::io::Write + Send>;

pub use geometry::{Component, Geometry, ReadMate, ResolvedGeometry};
pub use mapper::{
    Bijection, CrisprMapper, GexMapper, Library, Mapper, ProbeMapper, Ready, UmiMapper,
    Unpositioned, WhitelistMapper,
};
pub use processor::MapProcessor;
pub use run::{run_crispr, run_gex};
pub use utils::initialize_output_ibus;

pub const UMI_MIN_QUALITY: u8 = 10;
pub const ILLUMINA_QUALITY_OFFSET: u8 = 33;
