use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Error)]
pub enum BlockError {
    #[error("certificate already exists")]
    CertificateExists,
    #[error("{0}")]
    Other(String),
}
