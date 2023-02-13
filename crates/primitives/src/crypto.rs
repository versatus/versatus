use serde::Serialize;

pub type PublicKey = secp256k1::PublicKey;
pub type SecretKey = secp256k1::SecretKey;
pub type Signature = secp256k1::ecdsa::Signature;

/// Represents a byte slice produced from an instance of secp256k1::SecretKey
pub type SerializedSecretKey = Vec<u8>;

/// Represents a byte slice produced from an instance of secp256k1::PublicKey
pub type SerializedPublicKey = Vec<u8>;

/// Represents a String produced from an instance of secp256k1::PublicKey
pub type SerializedPublicKeyString = String;

#[derive(Debug, Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub enum SignatureType {
    PartialSignature,
    ThresholdSignature,
    ChainLockSignature,
}
