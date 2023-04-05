pub mod http;
pub mod indexer;

// use reqwest::Error as ReqwestError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reqwest error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("url error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("{0}")]
    Other(String),
}
