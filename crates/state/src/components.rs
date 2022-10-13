//FEATURE TAG(S): Left-Right Database, Left-Right State Trie
use std::fs;

/// This module contains the Network State struct (which will be replaced with
/// the Left-Right State Trie)
use accountable::accountable::Accountable;
use claim::claim::Claim;
use ledger::ledger::Ledger;
use log::info;
use noncing::nonceable::Nonceable;
use ownable::ownable::Ownable;
use reward::reward::{Reward, RewardState};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;

use crate::types::{
    StateArchive, StateBlockchain, StateChildBlock, StateGenesisBlock, StateLedger,
    StateNetworkState, StateParentBlock,
};

/// The components required for a node to sync with the network state
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Components {
    pub genesis: StateGenesisBlock,
    pub child: StateChildBlock,
    pub parent: StateParentBlock,
    pub blockchain: StateBlockchain,
    pub ledger: StateLedger,
    pub network_state: StateNetworkState,
    pub archive: StateArchive,
}

impl Components {
    /// Serializes the Components struct into a vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    /// Deserializes the Components struct from a byte array
    pub fn from_bytes(data: &[u8]) -> Components {
        serde_json::from_slice::<Components>(data).unwrap()
    }

    /// Serializes the Components struct into a string
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Deserializes the Components struct from a string.
    pub fn from_string(string: &String) -> Components {
        serde_json::from_str::<Components>(&string).unwrap()
    }
}
