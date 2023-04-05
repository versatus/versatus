use std::net::AddrParseError;

use network::config::BroadcastError;
use thiserror::Error;
use tokio::sync::mpsc::error::TryRecvError;
use theater::TheaterError;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("invalid node type {0} provided")]
    InvalidNodeType(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    AddrParse(#[from] AddrParseError),

    #[error("{0}")]
    Storage(#[from] vrrb_core::storage_utils::StorageError),

    #[error("{0}")]
    Broadcast(#[from] BroadcastError),

    #[error("{0}")]
    TryRecv(#[from] TryRecvError),

    #[error("{0}")]
    Event(#[from] events::Error),

    #[error("{0}")]
    Core(#[from] vrrb_core::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;

impl From<NodeError> for TheaterError {
    fn from(err: NodeError) -> Self {
        TheaterError::Other(err.to_string())
    }
}
