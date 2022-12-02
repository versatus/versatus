#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("chain proposed is shorter than local chain")]
    NotTallestChain,
    #[error("block out of sequence")]
    BlockOutOfSequence,
    #[error("invalid claim")]
    InvalidClaim,
    #[error("invalid last hash")]
    InvalidLastHash,
    #[error("invalid state hash")]
    InvalidStateHash,
    #[error("invalid block height")]
    InvalidBlockHeight,
    #[error("invalid block nonce")]
    InvalidBlockNonce,
    #[error("invalid block reward")]
    InvalidBlockReward,
    #[error("invalid txns in block")]
    InvalidTxns,
    #[error("invalid claim pointers")]
    InvalidClaimPointers,
    #[error("invalid next block reward")]
    InvalidNextBlockReward,
    #[error("invalid block signature")]
    InvalidBlockSignature,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ChainError>;
