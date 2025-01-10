pub mod crispr;
pub mod flex;
mod generic;
mod utils;

use generic::{ibu_map_pairs, ibu_map_probed_pairs};

#[cfg(feature = "binseq")]
mod generic_binseq;
#[cfg(feature = "binseq")]
use generic_binseq::ibu_map_pairs_binseq;
