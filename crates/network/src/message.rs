use serde::{Deserialize, Serialize};
use telemetry::info;

use crate::{
    components::StateComponent,
    packet::{NotCompleteError, Packet, Packetize},
};

pub const MAX_TRANSMIT_SIZE: usize = 1024;
pub const PROPOSAL_EXPIRATION_KEY: &str = "expires";
pub const PROPOSAL_YES_VOTE_KEY: &str = "yes";
pub const PROPOSAL_NO_VOTE_KEY: &str = "no";

/// The message struct contains the basic data contained in a message
/// sent across the network. This can be packed into bytes.
//TODO: Convert the Vec<u8> and u8's into custom types that are more
// descriptive of their purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Vec<u8>,
    pub source: Option<Vec<u8>>,
    pub data: Vec<u8>,
    pub sequence_number: Option<Vec<u8>>,
    pub signature: Option<Vec<u8>>,
    pub topics: Option<Vec<u8>>,
    pub key: Option<Vec<u8>>,
    pub validated: u8,
    pub return_receipt: u8,
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
        // TODO: handle this unwrap
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    /// Deserializes a byte array into a Message struct
    pub fn from_bytes(data: &[u8]) -> Message {
        serde_json::from_slice::<Message>(data).unwrap()
    }
}
