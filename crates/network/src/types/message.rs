use std::{collections::HashMap, net::SocketAddr};

use primitives::{NodeType, PublicKey};
use serde::{Deserialize, Serialize};
use udp2p::node::peer_id::PeerId;
use uuid::Uuid;
use vrrb_core::event_router::{Event, PeerData};

use crate::packet::{NotCompleteError, Packet, Packetize};

pub type MessageId = Uuid;
pub type MessageContents = Vec<u8>;

pub const MAX_TRANSMIT_SIZE: usize = 1024;
pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateBlock(pub u128);

/// Message types are the different types of messages that can be
/// packed and sent across the network.
//TODO: Convert Vec<u8>, String, u128 and other standard types with custom types
// that better describe their purpose
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageType {
    // TxnMessage {
    //     txn: Vec<u8>,
    //     sender_id: String,
    // },
    // TxnValidatorMessage {
    //     txn_validator: Vec<u8>,
    //     sender_id: String,
    // },
    // BlockMessage {
    //     block: Vec<u8>,
    //     sender_id: String,
    // },
    // ClaimMessage {
    //     claim: Vec<u8>,
    //     sender_id: String,
    // },
    // GetNetworkStateMessage {
    //     sender_id: String,
    //     requested_from: String,
    //     requestor_address: SocketAddr,
    //     requestor_node_type: Vec<u8>,
    //     lowest_block: u128,
    //     component: Vec<u8>,
    // },
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
    StateComponentsMessage {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    GenesisMessage {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    ChildMessage {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    ParentMessage {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    LedgerMessage {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    NetworkStateMessage {
        data: Vec<u8>,
        requestor: String,
        requestor_id: String,
        sender_id: String,
    },
    ClaimAbandonedMessage {
        claim: Vec<u8>,
        sender_id: String,
    },
    DKGPartCommitmentMessage {
        dkg_part_commitment: Vec<u8>,
        sender_id: String,
    },
    DKGACKCommitmentMessage {
        dkg_ack_commitment: Vec<u8>,
        sender_id: String,
    },
    SendPeerIdMessage {
        pub_key: String,
        peer_id: PeerId,
    },
    ResetPeerConnectionMessage {
        peer_id: PeerId,
    },
    RemovePeerMessage {
        peer_id: PeerId,
        socket_addr: SocketAddr,
    },
    AddPeerMessage {
        peer_id: PeerId,
        socket_addr: SocketAddr,
    },
    // SendChainLockSignatureMessage {
    //     chain_lock_signature: Vec<u8>,
    // },
}

impl MessageType {
    /// Serialize a message to into a vector of bytes
    pub fn as_bytes(self) -> Vec<u8> {
        serde_json::to_string(&self).unwrap().as_bytes().to_vec()
    }

    /// Deserialize a vector of bytes into a MessageType
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
    fn into_message(self, return_receipt: u8) -> Message {
        let id = Uuid::new_v4();
        Message {
            id,
            source: None,
            data: self.as_bytes(),
            sequence_number: None,
            return_receipt,
        }
    }
}
/// The message struct contains the basic data contained in a message
/// sent across the network. This can be packed into bytes.
//TODO: Convert the Vec<u8> and u8's into custom types that are more
// descriptive of their purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
// TODO: Replace message contents with an instance of MessageBody
pub struct Message {
    pub id: MessageId,
    pub data: MessageContents,
    pub source: Option<Vec<u8>>,
    pub sequence_number: Option<Vec<u8>>,
    pub return_receipt: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
// TODO: refactor MessageType into this
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
    DKGPartCommitment {
        dkg_part_commitment: Vec<u8>,
        sender_id: String,
    },
    DKGACKCommitment {
        dkg_ack_commitment: Vec<u8>,
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
        // pub_key: PublicKey,
        peer_id: primitives::types::PeerId,
        socket_addr: SocketAddr,
        node_type: NodeType,
    },
    Empty,
}

impl From<Vec<u8>> for MessageBody {
    fn from(data: Vec<u8>) -> Self {
        serde_json::from_slice::<MessageBody>(&data).unwrap_or(MessageBody::Empty)
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
    /// Serializes a Message struct into a vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.clone().into()
    }

    /// Deserializes a byte array into a Message struct
    pub fn from_bytes(data: &[u8]) -> Message {
        Self::from(data.to_vec())
    }
}

/// Represents an empty, often invalid Message
pub const NULL_MESSAGE: Message = Message {
    id: uuid::Uuid::nil(),
    data: vec![],
    source: None,
    sequence_number: None,
    return_receipt: 0,
};

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        serde_json::from_slice::<Message>(&data).unwrap_or(NULL_MESSAGE)
    }
}

impl From<Message> for Vec<u8> {
    fn from(msg: Message) -> Self {
        serde_json::to_vec(&msg).unwrap_or_default()
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

/// Converts a message into a vector of packets to be sent across
/// the transport layer.
impl Packetize for Message {
    type FlatPackets = Vec<u8>;
    type PacketBytes = Vec<Vec<u8>>;
    type PacketMap = HashMap<u32, Packet>;
    type Packets = Vec<Packet>;

    fn into_packets(self) -> Vec<Packet> {
        vec![]

        // let message_string = serde_json::to_string(&self).unwrap();
        // let message_bytes = message_string.as_bytes();
        // let n_bytes = message_bytes.len();
        // if n_bytes > MAX_TRANSMIT_SIZE {
        //     let mut n_packets = n_bytes / MAX_TRANSMIT_SIZE;
        //     if n_bytes % MAX_TRANSMIT_SIZE != 0 {
        //         n_packets += 1;
        //     }
        //     let mut end = MAX_TRANSMIT_SIZE;
        //     let mut start = 0;
        //     let range: Vec<_> = (0..n_packets).collect();
        //     let packets = range
        //         .iter()
        //         .map(|idx| {
        //             if *idx == n_packets - 1 {
        //                 start = end;
        //                 Packet::new(
        //                     self.id.clone(),
        //                     self.source.clone(),
        //                     message_bytes[start..].to_vec(),
        //                     (n_bytes - end).to_be_bytes().to_vec(),
        //                     (idx + 1).to_be_bytes().to_vec(),
        //                     n_packets.to_be_bytes().to_vec(),
        //                     self.return_receipt,
        //                 )
        //             } else if *idx == 0 {
        //                 Packet::new(
        //                     self.id.clone(),
        //                     self.source.clone(),
        //                     message_bytes[start..end].to_vec(),
        //                     MAX_TRANSMIT_SIZE.to_be_bytes().to_vec(),
        //                     (idx + 1).to_be_bytes().to_vec(),
        //                     n_packets.to_be_bytes().to_vec(),
        //                     self.return_receipt,
        //                 )
        //             } else {
        //                 start = end;
        //                 end = start + (MAX_TRANSMIT_SIZE);
        //                 Packet::new(
        //                     self.id.clone(),
        //                     self.source.clone(),
        //                     message_bytes[start..end].to_vec(),
        //                     MAX_TRANSMIT_SIZE.to_be_bytes().to_vec(),
        //                     (idx + 1).to_be_bytes().to_vec(),
        //                     n_packets.to_be_bytes().to_vec(),
        //                     self.return_receipt,
        //                 )
        //             }
        //         })
        //         .collect::<Vec<Packet>>();
        //
        //     packets
        // } else {
        //     let n_packets = 1usize;
        //     vec![Packet {
        //         id: self.id.clone(),
        //         source: self.source.clone(),
        //         data: message_bytes.to_vec(),
        //         size: n_bytes.to_be_bytes().to_vec(),
        //         packet_number: n_packets.to_be_bytes().to_vec(),
        //         total_packets: n_packets.to_be_bytes().to_vec(),
        //         return_receipt: self.return_receipt,
        //     }]
        // }
    }

    /// Serializes a vector of packets into nested vectors of bytes.
    fn as_packet_bytes(&self) -> Vec<Vec<u8>> {
        let packets = self.clone().into_packets();

        packets
            .iter()
            .map(|packet| packet.as_bytes())
            .collect::<Vec<Vec<u8>>>()
    }

    /// Reassembles a map of packets into a serialized vector of bytes that
    /// cab be converted back into a Message for processing
    fn assemble(map: &mut Self::PacketMap) -> Self::FlatPackets {
        let mut byte_slices = map
            .iter()
            .map(|(packet_number, packet)| (*packet_number, packet.clone()))
            .collect::<Vec<(u32, Packet)>>();

        byte_slices.sort_unstable_by_key(|k| k.0);
        let mut assembled = vec![];
        byte_slices.iter().for_each(|(_, v)| {
            assembled.extend(v.data.clone());
        });

        assembled
    }

    /// Does the same thing as assemble but with better error handling in the
    /// event packets are missing or cannot be assembled.
    fn try_assemble(map: &mut Self::PacketMap) -> Result<Self::FlatPackets, NotCompleteError> {
        if let Some((_, packet)) = map.clone().iter().next() {
            if map.len() == usize::from_be_bytes(packet.clone().convert_total_packets()) {
                let mut byte_slices = map
                    .iter()
                    .map(|(packet_number, packet)| (*packet_number, packet.clone()))
                    .collect::<Vec<(u32, Packet)>>();

                byte_slices.sort_unstable_by_key(|k| k.0);
                let mut assembled = vec![];

                byte_slices.iter().for_each(|(_, v)| {
                    assembled.extend(v.data.clone());
                });

                return Ok(assembled);
            }
        }
        Err(NotCompleteError)
    }
}
