use jsonrpsee::core::Error as RpseeError;
use std::net::SocketAddr;

pub mod http;
pub mod rpc;

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("jsonrpsee error: {0}")]
    JsonRpseeError(#[from] RpseeError),
}
