use serde::{Deserialize, Serialize};

use crate::Digest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message<D> {
    pub data: D,
    pub digest: Digest,
}
