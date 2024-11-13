use serde::{Deserialize, Deserializer};

// Helper function to deserialize strings into Vec<u8>
pub fn string_to_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(s.into_bytes())
}
