use std::net::SocketAddr;

pub mod http;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("invalid address provided: {0}")]
    InvalidAddr(SocketAddr),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ApiError>;
