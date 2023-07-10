use serde::{Deserialize, Serialize};

/// The unit of time within VRRB.
/// It lasts for some number
pub type Epoch = u128;
pub type Round = u128;
pub type Seed = u64;
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
pub const DEFAULT_VRRB_DB_PATH: &str = ".vrrb/node/db";
pub const DEFAULT_VRRB_WALLET_DATA_DIR_PATH: &str = ".vrrb/wallet";
pub const DEFAULT_CONNECTION_TIMEOUT_IN_SECS: u64 = 2;
pub const RAPTOR_DECODER_CACHE_LIMIT: usize = 10000;
pub const RAPTOR_DECODER_CACHE_TTL_IN_SECS: u64 = 1800000;

pub type RaptorUdpPort = u16;
pub type ByteVec = Vec<u8>;
pub type ByteSlice<'a> = &'a [u8];
pub type PayloadHash = ByteVec;
pub type RawSignature = ByteVec;
pub type PeerId = ByteVec;
pub type KademliaPeerId = kademlia_dht::Key;
pub type FarmerId = ByteVec;
pub type IsTxnValid = bool;
pub type PublicKeyShareVec = ByteVec;

#[deprecated(note = "Use TransactionDigest instead")]
pub type TxHash = ByteVec;

#[deprecated(note = "Use TransactionDigest and call to_string on it instead")]
pub type TxHashString = String;

#[deprecated(note = "Use Digest instead")]
pub type BlockHash = ByteVec;

pub type GroupPublicKey = ByteVec;

#[derive(Serialize, Deserialize, Hash, Clone, Debug, Eq, PartialEq)]
pub enum QuorumType {
    Farmer,
    Harvester,
}

pub type QuorumSize = usize;
pub type QuorumThreshold = usize;
pub type FarmerQuorumThreshold = usize;
pub type HarvesterQuorumThreshold = usize;

pub type NodeTypeBytes = ByteVec;
pub type QuorumPublicKey = ByteVec;
pub type PKShareBytes = ByteVec;
pub type PayloadBytes = ByteVec;
pub type LastBlockHeight = u128;
pub type PartCommitmentBytes = ByteVec;
pub type AckBytes = ByteVec;
pub type SenderId = u16;
pub type CurrentNodeId = u16;
