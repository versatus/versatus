use std::net::AddrParseError;

// use dkg_engine::DkgError;
use dyswarm::types::DyswarmError;
use events::EventMessage;
use miner::result::MinerError;
use theater::TheaterError;
use thiserror::Error;
use tokio::sync::mpsc::error::TryRecvError;
use vrrb_core::claim::ClaimError;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("invalid node type {0} provided")]
    InvalidNodeType(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    AddrParse(#[from] AddrParseError),

    #[error("{0}")]
    CoreStorage(#[from] vrrb_core::storage_utils::StorageError),

    #[error("{0}")]
    Storage(#[from] storage::storage_utils::StorageError),

    #[error("{0}")]
    TryRecv(#[from] TryRecvError),

    #[error("{0}")]
    BroadcastSend(#[from] tokio::sync::broadcast::error::SendError<EventMessage>),

    #[error("{0}")]
    MpscSend(#[from] tokio::sync::mpsc::error::SendError<EventMessage>),

    #[error("{0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("{0}")]
    JsonRpc(#[from] vrrb_rpc::ApiError),

    #[error("{0}")]
    Messr(#[from] messr::Error),

    #[error("{0}")]
    Dyswarm(#[from] dyswarm::types::DyswarmError),

    #[error("Error while creating instance of miner: {0}")]
    Miner(#[from] MinerError),

    #[error("Error while creating claim for node: {0}")]
    Claim(#[from] ClaimError),

    // #[error("DKG error: {0}")]
    // Dkg(#[from] DkgError),
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

impl From<NodeError> for DyswarmError {
    fn from(err: NodeError) -> Self {
        DyswarmError::Other(err.to_string())
    }
}
