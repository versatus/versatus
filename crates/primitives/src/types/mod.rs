use std::str::FromStr;

use serde::{Deserialize, Serialize};
pub type NodeId = String;
pub type NodeIdx = u16;
pub type NodeIdentifier = String;
pub type SecretKey = Vec<u8>;
pub type PublicKey = Vec<u8>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
#[serde(try_from = "String")]
pub enum NodeType {
    /// A Node that can archive, validate and mine tokens
    Full = 0,
    /// Same as `NodeType::Full` but without archiving capabilities
    Light = 1,
    /// Archives all transactions processed in the blockchain
    Archive = 2,
    /// Mining node
    Miner = 3,
    Bootstrap = 4,
    Validator = 5,
    MasterNode = 6,
    Unknown = 100,
}

impl FromStr for NodeType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "full" => Ok(NodeType::Full),
            "light" => Ok(NodeType::Light),
            "archive" => Ok(NodeType::Archive),
            "miner" => Ok(NodeType::Miner),
            "bootstrap" => Ok(NodeType::Bootstrap),
            "validator" => Ok(NodeType::Validator),
            "masternode" => Ok(NodeType::MasterNode),
            _ => Err(Error::Other("invalid node type".into())),
        }
    }
}

impl From<String> for NodeType {
    fn from(src: String) -> Self {
        match src.to_ascii_lowercase().as_str() {
            "full" => NodeType::Full,
            "light" => NodeType::Light,
            "archive" => NodeType::Archive,
            "miner" => NodeType::Miner,
            "bootstrap" => NodeType::Bootstrap,
            "validator" => NodeType::Validator,
            "masternode" => NodeType::MasterNode,
            _ => NodeType::Unknown,
        }
    }
}

impl From<usize> for NodeType {
    fn from(node_type: usize) -> Self {
        match node_type {
            0 => NodeType::Full,
            1 => NodeType::Light,
            2 => NodeType::Archive,
            3 => NodeType::Miner,
            4 => NodeType::Bootstrap,
            5 => NodeType::Validator,
            6 => NodeType::MasterNode,
            _ => NodeType::Unknown,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct StopSignal;

//TXN Hash or Block Hash
pub type Hash = Vec<u8>;
pub type RawSignature = Vec<u8>;

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
