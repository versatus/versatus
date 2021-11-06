use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;

#[derive(Debug)]
pub struct NotCompleteError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub id: Vec<u8>,
    pub source: Option<Vec<u8>>,
    pub data: Vec<u8>,
    pub size: Vec<u8>,
    pub packet_number: Vec<u8>,
    pub total_packets: Vec<u8>,
}

impl Packet {
    pub fn new(
        id: Vec<u8>,
        source: Option<Vec<u8>>,
        data: Vec<u8>,
        size: Vec<u8>,
        packet_number: Vec<u8>,
        total_packets: Vec<u8>,
    ) -> Packet {
        Packet {
            id,
            source,
            data,
            size,
            packet_number,
            total_packets,
        }
    }

    pub fn convert_packet_number(self) -> [u8; 8] {
        self.packet_number
            .try_into()
            .unwrap_or_else(|_| panic!("Expected a Vec of length 8"))
    }

    pub fn convert_total_packets(self) -> [u8; 8] {
        self.total_packets
            .try_into()
            .unwrap_or_else(|_| panic!("Expected a Vec of length 8"))
    }

    pub fn is_complete(&self) -> bool {
        usize::from_be_bytes(self.clone().convert_total_packets()) == 1
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes().to_vec()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_bytes(data: &[u8]) -> Packet {
        serde_json::from_slice(data).unwrap()
    }

    pub fn from_str(data: &str) -> Packet {
        serde_json::from_str(data).unwrap()
    }
}

pub trait Packetize {
    fn into_packets(&self) -> Vec<Packet>;
    fn as_packet_bytes(&self) -> Vec<Vec<u8>>;
    fn assemble(map: &mut HashMap<u32, Packet>) -> Vec<u8>;
    fn try_assemble(map: &mut HashMap<u32, Packet>) -> Result<Vec<u8>, NotCompleteError>;
}

impl Error for NotCompleteError {}

impl std::fmt::Display for NotCompleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "NotCompleteError")
    }
}
