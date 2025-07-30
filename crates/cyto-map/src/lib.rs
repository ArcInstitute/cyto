pub mod crispr;
pub mod generic;
pub mod gex;
mod implementor;
mod implementor_probe;
mod utils;

use implementor::{ibu_map_pairs_binseq, ibu_map_pairs_paraseq};
use implementor_probe::{ibu_map_probed_pairs_binseq, ibu_map_probed_pairs_paraseq};
