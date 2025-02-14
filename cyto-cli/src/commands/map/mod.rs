pub mod crispr;
pub mod flex;
mod generic;
mod generic_probe;
mod utils;

use generic::ibu_map_pairs_paraseq;
use generic_probe::ibu_map_probed_pairs_paraseq;

#[cfg(feature = "binseq")]
use generic::ibu_map_pairs_binseq;
#[cfg(feature = "binseq")]
use generic_probe::ibu_map_probed_pairs_binseq;
