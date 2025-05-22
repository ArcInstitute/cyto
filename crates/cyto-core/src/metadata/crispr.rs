use crate::aliases::{Anchor, Name, Sequence};
use crate::io::utils::string_to_bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Guide {
    #[serde(deserialize_with = "string_to_bytes")]
    pub name: Name,
    #[serde(deserialize_with = "string_to_bytes")]
    pub anchor: Anchor,
    #[serde(deserialize_with = "string_to_bytes")]
    pub sequence: Sequence,
}
