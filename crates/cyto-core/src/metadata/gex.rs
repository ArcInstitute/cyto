use crate::aliases::{Name, Sequence};
use crate::io::utils::string_to_bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Gex {
    #[serde(deserialize_with = "string_to_bytes")]
    pub unit_name: Name,
    #[serde(deserialize_with = "string_to_bytes")]
    pub aggr_name: Name,
    #[serde(deserialize_with = "string_to_bytes")]
    pub sequence: Sequence,
}
