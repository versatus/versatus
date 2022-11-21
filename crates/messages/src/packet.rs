//FEATURE TAGS: Packet Structure, P2P Network
use std::{convert::TryInto, error::Error};

use serde::{Deserialize, Serialize};

/// A Basic error unit struct to return in the event a series of packets cannot
/// be reassembled into a type
#[derive(Debug)]
pub struct NotCompleteError;

/// The basic structure that is converted into bytes to be sent across the
/// network
//TODO: Replace standard types with custom types to make it more obvious what their
// purposes are.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub id: Vec<u8>,
    pub source: Option<Vec<u8>>,
    pub data: Vec<u8>,
    pub size: Vec<u8>,
    pub packet_number: Vec<u8>,
    pub total_packets: Vec<u8>,
    pub return_receipt: u8,
}

impl Packet {
    /// Assembles and returns a new packet
    //TODO: Convert Vec<u8> and other standard types to custom types that are more
    // descriptive of their purpose
    pub fn new(
        id: Vec<u8>,
        source: Option<Vec<u8>>,
        data: Vec<u8>,
        size: Vec<u8>,
        packet_number: Vec<u8>,
        total_packets: Vec<u8>,
        return_receipt: u8,
    ) -> Packet {
        Packet {
            id,
            source,
            data,
            size,
            packet_number,
            total_packets,
            return_receipt,
        }
    }

    /// Converts a packet number into an array of bytes (8 bytes)
    pub fn convert_packet_number(self) -> [u8; 8] {
        self.packet_number
            .try_into()
            .unwrap_or_else(|_| panic!("Expected a Vec of length 8"))
    }

    /// Converts the total number of packets into an array of bytes (8 bytes)
    pub fn convert_total_packets(self) -> [u8; 8] {
        self.total_packets
            .try_into()
            .unwrap_or_else(|_| panic!("Expected a Vec of length 8"))
    }

    /// Returns true if the total number of packets is only 1
    pub fn is_complete(&self) -> bool {
        usize::from_be_bytes(self.clone().convert_total_packets()) == 1
    }

    /// Returns a vector of bytes from a Packet
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    /// Serializes a Packet into a string
    // TODO: Is this fine?
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Deserializes an array of bytes into a Packet
    pub fn from_bytes(data: &[u8]) -> Packet {
        serde_json::from_slice(data).unwrap()
    }

    /// Deserializes a string slice into a Packet
    // Is this ok?
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(data: &str) -> Packet {
        serde_json::from_str(data).unwrap()
    }
}

/// A trait to be implemented on anything that can be converted into a Packet or
/// from a Packet
pub trait Packetize {
    type Packets;
    type PacketBytes;
    type FlatPackets;
    type PacketMap;
    fn into_packets(self) -> Self::Packets;
    fn as_packet_bytes(&self) -> Self::PacketBytes;
    fn assemble(map: &mut Self::PacketMap) -> Self::FlatPackets;
    fn try_assemble(map: &mut Self::PacketMap) -> Result<Self::FlatPackets, NotCompleteError>;
}

/// Required to use `NotCompleteError` as an Error type in the Result enum
impl Error for NotCompleteError {}

/// Required to use `NotCompleteError` as an Error type in the Result enum
impl std::fmt::Display for NotCompleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "NotCompleteError")
    }
}
