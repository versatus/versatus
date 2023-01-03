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
    #[error("general invalid block error")]
    InvalidBlockHeader,
    #[error("invalid block header")]
    General,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub struct InvalidBlockError {
    pub reason: InvalidBlockErrorReason,
}

impl InvalidBlockError {
    pub fn new(reason: InvalidBlockErrorReason) -> Self {
        InvalidBlockError { reason }
    }
}

impl InvalidBlockErrorReason {
    pub fn to_str(&self) -> &str {
        match self {
            Self::General => "general invalid block",
            Self::BlockOutOfSequence => "block out of sequence",
            Self::InvalidBlockHeight => "invalid block height",
            Self::InvalidClaim => "invalid claim",
            Self::InvalidLastHash => "invalid last hash",
            Self::InvalidStateHash => "invalid state hash",
            Self::InvalidBlockNonce => "invalid block nonce",
            Self::InvalidBlockReward => "invalid block reward",
            Self::InvalidNextBlockReward => "invalid next block reward",
            Self::InvalidTxns => "invalid transactions within block",
            Self::InvalidClaimPointers => "invalid claim pointers",
            Self::InvalidBlockSignature => "invalid block signature",
            Self::NotTallestChain => "blockchain proposed is shorter than local chain",
            InvalidBlockErrorReason::NotTallestChain => todo!(),
            InvalidBlockErrorReason::BlockOutOfSequence => todo!(),
            InvalidBlockErrorReason::InvalidClaim => todo!(),
            InvalidBlockErrorReason::InvalidLastHash => todo!(),
            InvalidBlockErrorReason::InvalidStateHash => todo!(),
            InvalidBlockErrorReason::InvalidBlockHeight => todo!(),
            InvalidBlockErrorReason::InvalidBlockNonce => todo!(),
            InvalidBlockErrorReason::InvalidBlockReward => todo!(),
            InvalidBlockErrorReason::InvalidTxns => todo!(),
            InvalidBlockErrorReason::InvalidClaimPointers => todo!(),
            InvalidBlockErrorReason::InvalidNextBlockReward => todo!(),
            InvalidBlockErrorReason::InvalidBlockSignature => todo!(),
            InvalidBlockErrorReason::InvalidBlockHeader => todo!(),
            InvalidBlockErrorReason::General => todo!(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }
}

impl fmt::Display for InvalidBlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}
