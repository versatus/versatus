use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Clone, Serialize, Deserialize, Eq)]
pub enum SignerError {
    #[error("SignerError: Dkg state for node cannot be read")]
    DkgStateCannotBeRead,
    #[error("SignerError: Group public key missing for the quorum")]
    GroupPublicKeyMissing,
    #[error("SignerError: Secret key share for current master node is missing ")]
    SecretKeyShareMissing,
    #[error("SignerError: Error generating threshold signature: {0}")]
    ThresholdSignatureError(String),
    #[error("SignerError: Error generating partial signature: {0}")]
    PartialSignatureError(String),
    #[error("SignerError: Error verifying signature: {0}")]
    SignatureVerificationError(String),
    #[error("SignerError: ")]
    CorruptSignatureShare(String),
}

pub type SignerResult<T> = Result<T, SignerError>;
