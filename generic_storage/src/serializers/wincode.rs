use crate::error::{Result, StorageError};

// wincode is a bincode-compatible serializer using in-place initialization
// for faster throughput      no intermediate staging buffers on deserialization.
pub struct Wincode;

impl Wincode {
    pub fn to_bytes<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        T: wincode::SchemaWrite<wincode::config::DefaultConfig, Src = T> + ?Sized,
    {
        wincode::serialize(value).map_err(|e| StorageError::Serialize(e.to_string()))
    }

    pub fn from_bytes<T>(&self, bytes: &[u8]) -> Result<T>
    where
        T: wincode::DeserializeOwned<Dst = T>,
    {
        wincode::deserialize(bytes).map_err(|e| StorageError::Deserialize(e.to_string()))
    }
}