pub mod node;

use serde::{Deserialize, Serialize};
pub const VALIDATOR_THRESHOLD: f64 = 0.60;

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

//TXN Hash or Block Hash
pub type Hash = Vec<u8>;
pub type RawSignature = Vec<u8>;
pub type PeerID = Vec<u8>;
pub type SecretKeyBytes =Vec<u8>;
pub type PublicKeyBytes =Vec<u8>;


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
