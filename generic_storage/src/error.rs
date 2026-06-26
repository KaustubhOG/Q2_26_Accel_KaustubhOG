use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("serialization failed: {0}")]
    Serialize(String),

    #[error("deserialization failed: {0}")]
    Deserialize(String),

    // Attempted load on an empty storage.
    #[error("storage is empty — call save() before load()")]
    Empty,
}

pub type Result<T> = std::result::Result<T, StorageError>;
