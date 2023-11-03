use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Block;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Error)]
pub enum BlockError {
    #[error("certificate already exists for block {0:?}")]
    CertificateExists(Block),
    #[error("{0}")]
    Other(String),
}
