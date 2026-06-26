use crate::error::{Result, StorageError};

/// Serializer backed by serde_json.
///
/// JSON is the obvious choice when the stored bytes must be human-readable
/// or interoperable with non-Rust consumers.  The size and speed cost over
/// binary formats is the trade-off.
pub struct Json;

impl Json {
    pub fn to_bytes<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| StorageError::Serialize(e.to_string()))
    }

    pub fn from_bytes<T: serde::de::DeserializeOwned>(&self, bytes: &[u8]) -> Result<T> {
        serde_json::from_slice(bytes).map_err(|e| StorageError::Deserialize(e.to_string()))
    }
}
