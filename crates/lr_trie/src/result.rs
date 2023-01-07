pub type Result<T> = std::result::Result<T, LeftRightTrieError>;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LeftRightTrieError {
    #[error("failed to deserialize value")]
    FailedToDeserializeValue,

    #[error("value not found for key {0:?}")]
    #[deprecated]
    NoValueForKey(Vec<u8>),

    #[error("value for key {0} not found")]
    NotFound(String),

    #[error("trie error")]
    FailedToGetValueForKey,

    #[error("{0}")]
    Other(String),
}
