use std::{
    collections::{HashMap, HashSet},
    error::Error,
    net::SocketAddr,
    str::FromStr,
};

use commands::command::Command;
use messages::{
    message::Message,
    message_types::MessageType,
    packet::{Packet, Packetize},
};
use secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{command_handler::CommandHandler, message_handler::MessageHandler, result::*};

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
    type Err = NodeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO: define node types thoroughly
        match s {
            "full" => Ok(NodeType::Full),
            "light" => Ok(NodeType::Light),
            _ => Err(NodeError::InvalidNodeType(s.into())),
        }
    }
}
