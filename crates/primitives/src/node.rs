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
pub enum NodeType {
    /// Catch-All node type that can serve as a validator and miner
    Full = 0,
    /// Bootstrap nodes can only serve as a bootstrap node to kickstart a
    /// network
    Bootstrap = 1,
    /// A Miner node can participate in miner elections to produce convergence
    /// blocks
    Miner = 2,
    /// A Validator node can participate in quorum elections to validate
    /// transactions, create proposal blocks and certify convergence blocks
    Validator = 3,

    MasterNode = 4,
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
            "miner" => Ok(NodeType::Miner),
            "bootstrap" => Ok(NodeType::Bootstrap),
            "validator" => Ok(NodeType::Validator),
            "master" | "masternode" => Ok(NodeType::MasterNode),
            _ => Err(Error::Other("invalid node type".into())),
        }
    }
}

impl From<String> for NodeType {
    fn from(src: String) -> Self {
        match src.to_ascii_lowercase().as_str() {
            "miner" => NodeType::Miner,
            "bootstrap" => NodeType::Bootstrap,
            "validator" => NodeType::Validator,
            "master" | "masternode" => NodeType::MasterNode,
            _ => NodeType::Full,
        }
    }
}

impl From<usize> for NodeType {
    fn from(node_type: usize) -> Self {
        match node_type {
            0 => NodeType::Full,
            1 => NodeType::Bootstrap,
            2 => NodeType::Miner,
            3 => NodeType::Validator,
            4 => NodeType::MasterNode,
            _ => NodeType::Full,
        }
    }
}
