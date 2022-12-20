use std::{
    collections::{HashMap, HashSet},
    error::Error,
    net::SocketAddr,
    str::FromStr,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::result::*;

//TODO:There needs to be different node types, this is probably not the right
// variants for the node types we will need in the network, needs to be
// discussed.
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NodeAuth {
    // Builds a full block archive all blocks and all claims
    Archive,
    // Builds a Block Header archive and stores all claims
    Full,
    // Builds a Block Header and Claim Header archive. Maintains claims owned by this node. Can
    // mine blocks and validate transactions cannot validate claim exchanges.
    Light,
    // Stores last block header and all claim headers
    UltraLight,
    //TODO: Add a key field for the bootstrap node, sha256 hash of key in bootstrap node must ==
    // a bootstrap node key.
    Bootstrap,
}

impl NodeAuth {
    /// Serializes the NodeAuth variant it is called on into a vector of bytes.
    pub fn as_bytes(&self) -> Result<Vec<u8>> {
        // serde_json::to_string(self).unwrap().as_bytes().to_vec()
        Ok(serde_json::to_string(self)
            .map_err(|err| NodeError::Other(err.to_string()))?
            .as_bytes()
            .to_vec())
    }
}
