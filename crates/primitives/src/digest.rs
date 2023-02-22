use std::fmt::{Display, Formatter};

use secp256k1::hashes::hex;
use serde::{Deserialize, Serialize};

use crate::{ByteSlice, ByteVec};

/// For SHA-256
pub const DIGEST_LENGTH: usize = 32;

/// Represents a SHA-256 digest produced from any serializable data type
#[derive(Debug, Default, Clone, Copy, Hash, Deserialize, Serialize, Eq, PartialEq)]
pub struct Digest([u8; DIGEST_LENGTH]);

impl From<ByteVec> for Digest {
    fn from(byte_vec: ByteVec) -> Self {
        let converted = byte_vec.try_into().unwrap_or_default();

        Self(converted)
    }
}

impl<'a> From<ByteSlice<'a>> for Digest {
    fn from(byte_slice: ByteSlice) -> Self {
        let converted = byte_slice.try_into().unwrap_or_default();

        Self(converted)
    }
}

impl Display for Digest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        hex::format_hex(&self.0, f)
    }
}

impl Digest {
    /// Returns the raw bytes of the digest
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
