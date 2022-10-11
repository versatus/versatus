use std::net::AddrParseError;

use thiserror::Error;
use tokio::sync::mpsc::error::TryRecvError;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("invalid node type {0} provided")]
    InvalidNodeType(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    AddrParse(#[from] AddrParseError),

    #[error("{0}")]
    Storage(#[from] storage::StorageError),

    #[error("{0}")]
    TryRecv(#[from] TryRecvError),

    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;
