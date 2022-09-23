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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeType {
    /// A Node that can archive, validate and mine tokens
    Full,
    /// Same as `NodeType::Full` but without archiving capabilities
    Light,
    /// Archives all transactions processed in the blockchain
    Archive,
    /// Mining node
    Miner,
    Bootstrap,
    Validator,
    MasterNode,
}

impl FromStr for NodeType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO: define node types thoroughly
        match s {
            "full" => Ok(NodeType::Full),
            "light" => Ok(NodeType::Light),
            _ => Err(Error::Other("invalid node type".into())),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StopSignal;

//TXN Hash or Block Hash
pub type Hash = Vec<u8>;
pub type RawSignature = Vec<u8>;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {}
