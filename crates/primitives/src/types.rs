use secp256k1::{hashes::sha256, rand::rngs::OsRng, Secp256k1};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

pub type ByteVec = Vec<u8>;
pub type ByteSlice<'a> = &'a [u8];

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

type Hash = Vec<u8>;

// NOTE: will be replaced by TxnHash eventually
pub type TxHash = Hash;

pub const TXN_DIGEST_LENGTH: usize = 32;
/// WIP structure that represents a transaction ID within VRRB, it has some
/// useful utility methods attached to it
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TxnHash([u8; TXN_DIGEST_LENGTH]);

impl From<ByteVec> for TxnHash {
    fn from(byte_vec: ByteVec) -> Self {
        let converted = byte_vec.try_into().unwrap_or_default();

        Self(converted)
    }
}

impl<'a> From<ByteSlice<'a>> for TxnHash {
    fn from(byte_slice: ByteSlice) -> Self {
        let converted = byte_slice.try_into().unwrap_or_default();

        Self(converted)
    }
}

pub type TxHashString = String;

pub type PayloadHash = Hash;
pub type BlockHash = Hash;
pub type RawSignature = Vec<u8>;
pub type PeerId = Vec<u8>;

/// Represents a byte slice produced from an instance of secp256k1::SecretKey
pub type SerializedSecretKey = Vec<u8>;

/// Represents a byte slice produced from an instance of secp256k1::PublicKey
pub type SerializedPublicKey = Vec<u8>;

/// Represents a String produced from an instance of secp256k1::PublicKey
pub type SerializedPublicKeyString = String;

pub type PublicKey = secp256k1::PublicKey;
pub type SecretKey = secp256k1::SecretKey;
pub type Signature = secp256k1::ecdsa::Signature;

/// Represents an account's public key serialized to bytes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountAddress(PublicKey);

impl AccountAddress {
    pub fn new(public_key: PublicKey) -> Self {
        Self(public_key)
    }
}

impl std::fmt::Display for AccountAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

pub type AccountKeypair = (secp256k1::SecretKey, secp256k1::PublicKey);

pub fn generate_account_keypair() -> AccountKeypair {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}

/// Represents a secp256k1 public key, hashed with sha256::digest
pub type Address = String;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum SignatureType {
    PartialSignature,
    ThresholdSignature,
    ChainLockSignature,
}

#[macro_export]
macro_rules! is_enum_variant {
    ($v:expr, $p:pat) => {
        if let $p = $v {
            true
        } else {
            false
        }
    };
}

/// The unit of time within VRRB.
/// It lasts for some number
pub type Epoch = u128;

pub const GENESIS_EPOCH: Epoch = 0;
pub const GROSS_UTILITY_PERCENTAGE: f64 = 0.01;
pub const PERCENTAGE_CHANGE_SUPPLY_CAP: f64 = 0.25;

// Time-related helper constants
pub const NANO: u128 = 1;
pub const MICRO: u128 = NANO * 1000;
pub const MILLI: u128 = MICRO * 1000;
pub const SECOND: u128 = MILLI * 1000;
pub const VALIDATOR_THRESHOLD: f64 = 0.60;

pub const NUMBER_OF_NETWORK_PACKETS: usize = 32;
pub const DEFAULT_VRRB_DATA_DIR_PATH: &str = ".vrrb";
