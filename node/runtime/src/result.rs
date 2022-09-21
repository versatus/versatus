use std::net::AddrParseError;

use node::NodeError;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    AddrParse(#[from] AddrParseError),

    #[error("{0}")]
    Node(#[from] NodeError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
