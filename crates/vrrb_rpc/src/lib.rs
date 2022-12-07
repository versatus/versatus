use jsonrpsee::core::Error as RpseeError;
use std::net::SocketAddr;

pub mod http;
pub mod rpc;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("jsonrpsee error: {0}")]
    JsonRpseeError(#[from] RpseeError),

    #[error("invalid address provided: {0}")]
    InvalidAddr(SocketAddr),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ApiError>;
