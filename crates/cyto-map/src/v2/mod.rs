mod geometry;
mod mapper;
mod processor;
mod run;
mod stats;
mod utils;

const REMAP_WINDOW: usize = 1;
const GEX_MAX_HDIST: usize = 3;
type BoxedWriter = Box<dyn std::io::Write + Send>;

pub use geometry::{Component, Geometry, ReadMate, ResolvedGeometry};
pub use mapper::{
    Bijection, CrisprMapper, GexMapper, Library, Mapper, ProbeMapper, Ready, UmiMapper,
    Unpositioned, WhitelistMapper,
};
pub use processor::MapProcessor;
pub use run::{run_crispr2, run_gex2};
pub use utils::initialize_output_ibus;

const GEOMETRY_GEX_FLEX_V1: &str = "[barcode][umi:12]|[gex][:18][probe]";
const GEOMETRY_CRISPR_PROPERSEQ: &str = "[barcode][umi:12]|[:18][probe][anchor][protospacer]";
