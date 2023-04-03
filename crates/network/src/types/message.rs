use std::net::SocketAddr;

use events::{Event, PeerData, Vote};
use primitives::{FarmerQuorumThreshold, NodeType, PublicKey, QuorumType};
use serde::{Deserialize, Serialize};
use udp2p::node::peer_id::PeerId;
use uuid::Uuid;
pub type MessageId = Uuid;
pub type MessageContents = Vec<u8>;

pub const MAX_TRANSMIT_SIZE: usize = 1024;
pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

/// Represents an empty, often invalid Message
pub const NULL_MESSAGE: Message = Message {
    id: uuid::Uuid::nil(),
    data: MessageBody::Empty,
    timestamp: 0,
};

/// The message struct contains the basic data contained in a message
/// sent across the network. This can be packed into bytes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub id: MessageId,
    pub data: MessageBody,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
// TODO: fix generic body type
// pub enum MessageBody<D: Serialize + Deserialize> {
pub enum MessageBody {
    InvalidBlock {
        block_height: u128,
        reason: Vec<u8>,
        miner_id: String,
        sender_id: String,
    },
    Disconnect {
        sender_id: String,
        pubkey: String,
    },
    StateComponents {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Genesis {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Child {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Parent {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    Ledger {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    NetworkState {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    ClaimAbandoned {
        claim: Vec<u8>,
        sender_id: String,
    },
    ResetPeerConnection {
        peer_id: PeerId,
    },
    RemovePeer {
        peer_id: PeerId,
        socket_addr: SocketAddr,
    },
    AddPeer {
        // TODO: figure out what to do with this public key field
        // pub_key: PublicKey,
        peer_id: primitives::PeerId,
        socket_addr: SocketAddr,
        node_type: NodeType,
    },
    DKGPartCommitment {
        part_commitment: Vec<u8>,
        sender_id: u16,
    },
    DKGPartAcknowledgement {
        curr_node_id: u16,
        sender_id: u16,
        ack: Vec<u8>,
    },
    Vote {
        vote: Vote,
        quorum_type: QuorumType,
        farmer_quorum_threshold: FarmerQuorumThreshold,
    },
    Empty,
}

impl Default for MessageBody {
    fn default() -> Self {
        MessageBody::Empty
    }
}

impl From<Vec<u8>> for MessageBody {
    fn from(data: Vec<u8>) -> Self {
        serde_json::from_slice::<MessageBody>(&data).unwrap_or_default()
    }
}

impl From<MessageBody> for Vec<u8> {
    fn from(body: MessageBody) -> Self {
        serde_json::to_vec(&body).unwrap_or_default()
    }
}

/// AsMessage is a trait that when implemented on a custom type allows
/// for the easy conversion of the type into a message that can be packed
/// into a byte array and sent across the network.
pub trait AsMessage {
    fn into_message(self, return_receipt: u8) -> Message;
}

impl Message {
    pub fn new(msg: MessageBody) -> Self {
        let timestamp = chrono::Utc::now().timestamp();

        Message {
            id: Uuid::new_v4(),
            data: msg,
            timestamp,
        }
    }

    /// Serializes a Message struct into a vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.clone().into()
    }

    /// Deserializes a byte array into a Message struct
    pub fn from_bytes(data: &[u8]) -> Message {
        Self::from(data.to_vec())
    }

    /// Deserializes a JSON-encoded byte array into a Message struct
    pub fn from_json_bytes(data: &[u8]) -> Message {
        serde_json::from_slice(data).unwrap_or_default()
    }

    /// Serializes a Message struct into a JSON-encoded vector of bytes
    pub fn as_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap_or_default()
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        bincode::deserialize(&data).unwrap_or_default()
    }
}

impl From<Message> for Vec<u8> {
    fn from(msg: Message) -> Self {
        bincode::serialize(&msg).unwrap_or_default()
    }
}

impl From<Event> for MessageBody {
    fn from(event: Event) -> Self {
        match event {
            Event::PeerJoined(data) => MessageBody::AddPeer {
                peer_id: data.peer_id,
                socket_addr: data.address,
                node_type: data.node_type,
            },

            _ => MessageBody::Empty,
        }
    }
}

impl From<MessageBody> for Event {
    fn from(body: MessageBody) -> Self {
        match body {
            MessageBody::Empty => Event::NoOp,
            MessageBody::AddPeer {
                peer_id,
                socket_addr,
                node_type,
            } => Event::PeerJoined(PeerData {
                address: socket_addr,
                node_type,
                peer_id,
            }),
            _ => Event::NoOp,
        }
    }
}
