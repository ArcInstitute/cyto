pub mod crispr;
pub mod flex;
// mod generic;
mod generic_paraseq;
mod generic_probe_paraseq;
mod utils;

use generic_paraseq::ibu_map_pairs_paraseq;
use generic_probe_paraseq::ibu_map_probed_pairs_paraseq;

#[cfg(feature = "binseq")]
mod generic_binseq;
#[cfg(feature = "binseq")]
use generic_binseq::ibu_map_pairs_binseq;
