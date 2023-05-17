/// A Basic error type to propagate in the event that there is no
/// valid miner uner the proof of claim algorithm
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MinerError {
    #[error("no lowest pointer: {0}")]
    NoLowestPointerError(String),
    #[error("Invalid signature during miner's claim verification")]
    InvalidSignature,
    #[error("Invalid public of miner encountered ")]
    InvalidPublicKey,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MinerError>;
