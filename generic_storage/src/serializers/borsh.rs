use crate::error::{Result, StorageError};

/// Serializer backed by the Borsh binary format.
///
/// Borsh was designed for deterministic, canonical encoding — useful when
/// the byte representation itself must be predictable (e.g. hashing, on-chain).
pub struct Borsh;

impl Borsh {
    pub fn to_bytes<T: borsh::BorshSerialize>(&self, value: &T) -> Result<Vec<u8>> {
        borsh::to_vec(value).map_err(|e| StorageError::Serialize(e.to_string()))
    }

    pub fn from_bytes<T: borsh::BorshDeserialize>(&self, bytes: &[u8]) -> Result<T> {
        T::try_from_slice(bytes).map_err(|e| StorageError::Deserialize(e.to_string()))
    }
}
