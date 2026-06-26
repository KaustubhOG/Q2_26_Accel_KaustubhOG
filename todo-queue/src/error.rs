use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    Serialize(String),

    #[error("deserialization failed: {0}")]
    Deserialize(String),

    #[error("queue is empty")]
    EmptyQueue,

    #[error("usage: todo <add <task>|list|done>")]
    BadArgs,
}

pub type Result<T> = std::result::Result<T, AppError>;
