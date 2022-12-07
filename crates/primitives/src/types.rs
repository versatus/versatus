use secp256k1::rand::rngs::OsRng;
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

type Hash = Vec<u8>;
pub type TxHash = Hash;
pub type PayloadHash = Hash;
pub type BlockHash = Hash;
pub type RawSignature = Vec<u8>;
pub type PeerId = Vec<u8>;

/// Represents a byte slice produced from an instance of secp256k1::SecretKey
pub type SerializedSecretKey = Vec<u8>;

/// Represents a byte slice produced from an instance of secp256k1::PublicKey
pub type SerializedPublicKey = Vec<u8>;

pub type PublicKey = secp256k1::PublicKey;
pub type SecretKey = secp256k1::SecretKey;

pub type AccountKeypair = (secp256k1::SecretKey, secp256k1::PublicKey);

pub fn generate_account_keypair() -> AccountKeypair {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}

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
