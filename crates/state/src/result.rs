use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum StateError {
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, StateError>;
