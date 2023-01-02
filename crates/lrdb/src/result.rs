use std::result::Result as StdResult;
use thiserror::Error;

pub type Nonce = u32;
pub type Result<T> = StdResult<T, LeftRightDbError>;

#[derive(Error, PartialEq, Eq, Debug)]
pub enum LeftRightDbError {
    #[error("record already exists")]
    RecordExists,

    #[error("entry {0} not found")]
    NotFound(String),

    #[error("unknown error occurred")]
    Unknown,

    #[error("{0}")]
    Other(String),

    #[error("account not found: {0}")]
    AccountNotFound(String),
}
