use crate::message::{AsMessage, Message};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateBlock(pub u128);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    TxnMessage {
        txn: Vec<u8>,
        sender_id: String,
    },
    TxnValidatorMessage {
        txn_validator: Vec<u8>,
        sender_id: String,
    },
    BlockMessage {
        block: Vec<u8>,
        sender_id: String,
    },
    ClaimMessage {
        claim: Vec<u8>,
        sender_id: String,
    },
    NeedBlocksMessage {
        blocks_needed: Vec<u128>,
        sender_id: String,
    },
    NeedBlockMessage {
        block_last_hash: String,
        sender_id: String,
        requested_from: String,
    },
    MissingBlock {
        block: Vec<u8>,
        requestor: String,
        sender_id: String,
    },
    GetNetworkStateMessage {
        sender_id: String,
        requested_from: String,
        requestor_address: SocketAddr,
        requestor_node_type: Vec<u8>,
        lowest_block: u128,
        component: Vec<u8>,
    },
    InvalidBlockMessage {
        block_height: u128,
        reason: Vec<u8>,
        miner_id: String,
        sender_id: String,
    },
    DisconnectMessage {
        sender_id: String,
        pubkey: String,
    },
    NeedGenesisBlock {
        sender_id: String,
        requested_from: String,
    },
    MissingGenesis {
        block: Vec<u8>,
        requestor: String,
        sender_id: String,
    },
    StateComponentsMessage {
        data: Vec<u8>,
        requestor: String,
        sender_id: String,
    },
    ClaimAbandonedMessage {
        claim: Vec<u8>,
        sender_id: String,
    }
}

impl MessageType {
    pub fn as_bytes(self) -> Vec<u8> {
        serde_json::to_string(&self).unwrap().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Option<MessageType> {
        if let Ok(message) = serde_json::from_slice::<MessageType>(data) {
            Some(message)
        } else {
            None
        }
    }
}

impl StateBlock {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }
}

impl AsMessage for MessageType {
    fn into_message(&self, return_receipt: u8) -> Message {
        Message {
            id: digest_bytes(&self.clone().as_bytes()).as_bytes().to_vec(),
            source: None,
            data: self.clone().as_bytes(),
            sequence_number: None,
            signature: None,
            topics: None,
            key: None,
            validated: 0,
            return_receipt,
        }
    }
}
