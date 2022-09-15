use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::packet::{NotCompleteError, Packet, Packetize};
pub const MAX_TRANSMIT_SIZE: usize = 1024;

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
    fn into_message(&self, return_receipt: u8) -> Message;
}

impl Message {
    /// Serializes a Message struct into a vector of bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    /// Deserializes a byte array into a Message struct
    pub fn from_bytes(data: &[u8]) -> Message {
        serde_json::from_slice::<Message>(data).unwrap()
    }
}

/// Converts a message into a vector of packets to be sent across
/// the transport layer.
impl Packetize for Message {
    type FlatPackets = Vec<u8>;
    type PacketBytes = Vec<Vec<u8>>;
    type PacketMap = HashMap<u32, Packet>;
    type Packets = Vec<Packet>;

    fn into_packets(&self) -> Vec<Packet> {
        let message_string = serde_json::to_string(self).unwrap();
        let message_bytes = message_string.as_bytes();
        let n_bytes = message_bytes.len();
        if n_bytes > MAX_TRANSMIT_SIZE {
            let mut n_packets = n_bytes / MAX_TRANSMIT_SIZE;
            if n_bytes % MAX_TRANSMIT_SIZE != 0 {
                n_packets += 1;
            }
            let mut end = MAX_TRANSMIT_SIZE;
            let mut start = 0;
            let range: Vec<_> = (0..n_packets).collect();
            let packets = range
                .iter()
                .map(|idx| {
                    if *idx == n_packets - 1 {
                        start = end;
                        return Packet::new(
                            self.id.clone(),
                            self.source.clone(),
                            message_bytes[start..].to_vec(),
                            (n_bytes - end).to_be_bytes().to_vec(),
                            (idx + 1).to_be_bytes().to_vec(),
                            n_packets.to_be_bytes().to_vec(),
                            self.return_receipt,
                        );
                    } else if *idx == 0 {
                        return Packet::new(
                            self.id.clone(),
                            self.source.clone(),
                            message_bytes[start..end].to_vec(),
                            MAX_TRANSMIT_SIZE.to_be_bytes().to_vec(),
                            (idx + 1).to_be_bytes().to_vec(),
                            n_packets.to_be_bytes().to_vec(),
                            self.return_receipt,
                        );
                    } else {
                        start = end;
                        end = start + (MAX_TRANSMIT_SIZE);
                        return Packet::new(
                            self.id.clone(),
                            self.source.clone(),
                            message_bytes[start..end].to_vec(),
                            MAX_TRANSMIT_SIZE.to_be_bytes().to_vec(),
                            (idx + 1).to_be_bytes().to_vec(),
                            n_packets.to_be_bytes().to_vec(),
                            self.return_receipt,
                        );
                    }
                })
                .collect::<Vec<Packet>>();

            packets
        } else {
            let n_packets = 1usize;
            vec![Packet {
                id: self.id.clone(),
                source: self.source.clone(),
                data: message_bytes.to_vec(),
                size: n_bytes.to_be_bytes().to_vec(),
                packet_number: n_packets.to_be_bytes().to_vec(),
                total_packets: n_packets.to_be_bytes().to_vec(),
                return_receipt: self.return_receipt,
            }]
        }
    }

    /// Serializes a vector of packets into nested vectors of bytes.
    fn as_packet_bytes(&self) -> Vec<Vec<u8>> {
        let packets = self.into_packets();

        packets
            .iter()
            .map(|packet| return packet.as_bytes())
            .collect::<Vec<Vec<u8>>>()
    }

    /// Reassembles a map of packets into a serialized vector of bytes that can
    /// be converted back into a Message for processing
    fn assemble(map: &mut Self::PacketMap) -> Self::FlatPackets {
        let mut byte_slices = map
            .iter()
            .map(|(packet_number, packet)| return (*packet_number, packet.clone()))
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
                    .map(|(packet_number, packet)| return (*packet_number, packet.clone()))
                    .collect::<Vec<(u32, Packet)>>();

                byte_slices.sort_unstable_by_key(|k| k.0);
                let mut assembled = vec![];

                byte_slices.iter().for_each(|(_, v)| {
                    assembled.extend(v.data.clone());
                });

                return Ok(assembled);
            }
        }
        return Err(NotCompleteError);
    }
}
