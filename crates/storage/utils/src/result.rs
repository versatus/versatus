#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),

    #[error("record already exists")]
    RecordExists,

    #[error("entry {0} not found")]
    NotFound(String),

    #[error("unknown error occurred")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, StorageError>;
