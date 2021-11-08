use crate::message::{AsMessage, Message};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateBlock(pub u128);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    NetworkStateDataBaseMessage {
        object: Vec<u8>,
        data: Vec<u8>,
        chunk_number: u32,
        total_chunks: u32,
        last_block: u128,
        requestor: String,
        sender_id: String,
    },
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
    BlockChunkMessage {
        sender_id: String,
        requestor: String,
        block_height: u128,
        chunk_number: u128,
        total_chunks: u128,
        data: Vec<u8>,
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
    },
    General {
        data: Vec<u8>,
    },
    Identify {
        data: String,
        pubkey: String,
    },
    NewPeer {
        data: Vec<u8>,
        pubkey: String,
    },
    KnownPeers {
        data: Vec<u8>,
    },
    AckMessage {
        packet_id: String,
        packet_number: u32,
        src: String,
    },
    FirstHolePunch {
        data: Vec<u8>,
        pubkey: String,
    },
    SecondHolePunch {
        data: Vec<u8>,
        pubkey: String,
    },
    FinalHolePunch {
        data: Vec<u8>,
        pubkey: String,
    },
    InitHandshake {
        data: Vec<u8>,
        pubkey: String,
        signature: String,
    },
    ReciprocateHandshake {
        data: Vec<u8>,
        pubkey: String,
        signature: String,
    },
    CompleteHandshake {
        data: Vec<u8>,
        pubkey: String,
        signature: String,
    },
    Ping {
        data: Vec<u8>,
        addr: Vec<u8>,
        timestamp: Vec<u8>,
    },
    Pong {
        data: Vec<u8>,
        addr: Vec<u8>,
        ping_timestamp: Vec<u8>,
        pong_timestamp: Vec<u8>,
    },
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
