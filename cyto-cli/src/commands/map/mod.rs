pub mod crispr;
pub mod flex;
mod generic;
mod generic_paraseq;
mod utils;

use generic::{ibu_map_pairs, ibu_map_probed_pairs};
use generic_paraseq::ibu_map_pairs_paraseq;

#[cfg(feature = "binseq")]
mod generic_binseq;
#[cfg(feature = "binseq")]
use generic_binseq::ibu_map_pairs_binseq;
