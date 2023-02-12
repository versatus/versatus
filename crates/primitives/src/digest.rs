use std::fmt::{Display, Formatter};

use secp256k1::hashes::hex;

use crate::{ByteSlice, ByteVec};

/// For SHA-256
pub const DIGEST_LENGTH: usize = 32;

/// Represents a SHA-256 digest produced from any serializable data type
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

pub const TRANSACTION_DIGEST_LENGTH: usize = DIGEST_LENGTH;

pub type TransactionDigest = Digest;

// pub type TxHashString = String;
// pub type PayloadHash = Hash;
// pub type BlockHash = Hash;
// pub type RawSignature = Vec<u8>;
// pub type PeerId = Vec<u8>;
// /// Represents a byte slice produced from an instance of secp256k1::SecretKey
// pub type SerializedSecretKey = Vec<u8>;
// /// Represents a byte slice produced from an instance of secp256k1::PublicKey
// pub type SerializedPublicKey = Vec<u8>;
// /// Represents a String produced from an instance of secp256k1::PublicKey
// pub type SerializedPublicKeyString = String;
// pub type PublicKey = secp256k1::PublicKey;
// pub type SecretKey = secp256k1::SecretKey;
// pub type Signature = secp256k1::ecdsa::Signature;
