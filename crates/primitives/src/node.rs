use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

// Represents a UUID serialized into a string
pub type NodeId = String;
pub type NodeIdx = u16;
pub type NodeIdentifier = String;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("invalid environment: {0}")]
    InvalidEnvironment(String),

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
// #[serde(try_from = "String")]
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
    // MasterNode = 6,
    // RPCNode = 7,
    Farmer = 8,
    Harvester = 9,
    // Unknown = 100,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
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
            "rpc" => Ok(NodeType::RPCNode),
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
            "farmer" => NodeType::Farmer,
            "rpc" => NodeType::RPCNode,
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
            7 => NodeType::RPCNode,
            8 => NodeType::Farmer,
            _ => NodeType::Unknown,
        }
    }
}
