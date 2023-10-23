use std::fmt::Display;

use crate::NodeId;
use crate::PublicKey;
use crate::Signature;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

pub const NETWORK_TOPIC_STR: &str = "network-events";
pub const RUNTIME_TOPIC_STR: &str = "runtime-events";
pub const JSON_RPC_API_TOPIC_STR: &str = "json-rpc-api-control";

pub type ByteVec = Vec<u8>;
pub type ByteSlice<'a> = &'a [u8];
pub type ByteSlice32Bit = [u8; 32];
pub type ByteSlice48Bit = [u8; 48];
pub type PayloadHash = ByteVec;
pub type RawSignature = ByteVec;
pub type PeerId = ByteVec;
pub type KademliaPeerId = kademlia_dht::Key;
pub type FarmerId = ByteVec;
pub type IsTxnValid = bool;
pub type PublicKeyShareVec = ByteVec;

#[derive(Serialize, Deserialize, Hash, Clone, Debug, Eq, PartialEq)]
pub enum TxnValidationStatus {
    Valid,
    Invalid,
}

// NOTE: change to the appropriate type when we have a consensus
pub type ProgramExecutionOutput = String;

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

impl std::fmt::Display for QuorumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuorumType::Farmer => f.write_str("Farmer"),
            QuorumType::Harvester => f.write_str("Harvester"),
        }
    }
}

#[derive(Serialize, Deserialize, Hash, Clone, Debug, Eq, PartialEq)]
pub struct ConvergencePartialSig {
    pub sig: Signature,
    pub block_hash: String,
    //TODO: add node_idx for checking sig along the way
    //pub node_idx: NodeIdx
}

pub type QuorumSize = usize;
pub type QuorumThreshold = usize;
pub type FarmerQuorumThreshold = usize;
pub type HarvesterQuorumThreshold = usize;
pub type QuorumPubKey = String;

pub type NodeTypeBytes = ByteVec;
pub type QuorumPublicKey = ByteVec;
pub type PKShareBytes = ByteVec;
pub type PayloadBytes = ByteVec;
pub type LastBlockHeight = u128;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QuorumKind {
    #[default]
    Harvester,
    Farmer,
    Miner,
}

impl Display for QuorumKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuorumKind::Harvester => write!(f, "Harvester"),
            QuorumKind::Farmer => write!(f, "Farmer"),
            QuorumKind::Miner => write!(f, "Miner"),
        }
    }
}

/// A hashed [PublicKeySet].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct QuorumId(String);

impl QuorumId {
    pub fn new(quorum_kind: QuorumKind, members: Vec<(NodeId, PublicKey)>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(quorum_kind.to_string().as_bytes());

        for (id, pubkey) in members.iter() {
            hasher.update(id.as_bytes());
            hasher.update(pubkey.serialize());
        }
        let result = hasher.finalize();

        Self(hex::encode(result))
    }
    pub fn get_inner(&self) -> String {
        self.0.clone()
    }
}
