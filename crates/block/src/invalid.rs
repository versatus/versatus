use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum InvalidBlockErrorReason {
    #[error("blockchain proposed shorter than local chain")]
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
    #[error("too many txns in block")]
    InvalidBlockSize,
    #[error("general invalid block error")]
    General,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub struct BlockError {
    pub reason: InvalidBlockErrorReason,
}

impl BlockError {
    pub fn new(reason: InvalidBlockErrorReason) -> Self {
        BlockError { reason }
    }
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}
