use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    error::Error,
    fs,
    fs::File,
    io::{Read, Result},
    net::{Ipv6Addr, SocketAddr},
    path::PathBuf,
    result::Result as StdResult,
    str,
    sync::Arc,
};

use block::Block;
use crossbeam_channel::{Receiver, Sender};
use futures::future::try_join_all;
use rand::{distributions::Alphanumeric, thread_rng, Rng, RngCore};
use raptorq::{Decoder, Encoder, EncodingPacket, ObjectTransmissionInformation};
use serde::{Deserialize, Serialize};
use telemetry::error;
use tokio::net::UdpSocket;
use vrrb_core::txn::Txn;

/// Maximum over-the-wire size of a Transaction
///   1280 is IPv6 minimum MTU
///   40 bytes is the size of the IPv6 header
///   8 bytes is the size of the fragment header

pub(crate) const MTU_SIZE: usize = 1280;

pub(crate) const BATCH_ID_SIZE: usize = 32;

const PACKET_SNO: usize = 4;

const FLAGS: usize = 1;

///Index at which actual payload starts.
const DECODER_DATA_INDEX: usize = 40;

///How many packets to recieve from socket in single system call
pub(crate) const NUM_RCVMMSGS: usize = 32;

///   40 bytes is the size of the IPv6 header
///   8 bytes is the size of the fragment header
///   True payload size ,or the size of single packet that will be written to or
/// read from socket
const PAYLOAD_SIZE: usize = MTU_SIZE - PACKET_SNO - BATCH_ID_SIZE - FLAGS - 40 - 8;

/// Below is the type that shall be used to broadcast RaptorQ Data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RaptorBroadCastedData {
    Block(Block),
}

/// It takes a batch id, a sequence number, and a payload, and returns a packet
///
/// Arguments:
///
/// * `batch_id`: This is the batch id that we're sending.
/// * `payload`: the data to be sent
///
/// Returns:
///
/// A vector of bytes
pub fn create_packet(symbol_size: u16, batch_id: [u8; BATCH_ID_SIZE], payload: Vec<u8>) -> Vec<u8> {
    let mut mtu: Vec<u8> = vec![];

    // empty byte for raptor coding length
    // doing the plus one since raptor is returning minus 1 length.

    mtu.push(0_u8);
    // forward-flag at the beginning
    mtu.push(1_u8);

    //Size of Payload

    mtu.extend((payload.len() as u32).to_le_bytes());

    mtu.extend(symbol_size.to_le_bytes());

    for id in batch_id {
        mtu.push(id);
    }
    mtu.extend_from_slice(&payload);
    mtu
}

/// `split_into_packets` takes a `full_list` of bytes, a `batch_id` and an
/// `erasure_count` and returns a `Vec<Vec<u8>>` of packets
///
/// Arguments:
///
/// * `full_list`: The list of bytes to be split into packets
/// * `batch_id`: This is a unique identifier for the batch of packets.
/// * `erasure_count`: The number of packets that can be lost and still be able
///   to recover the original data.
pub fn split_into_packets(
    full_list: &[u8],
    batch_id: [u8; BATCH_ID_SIZE],
    erasure_count: u32,
) -> Vec<Vec<u8>> {
    let packet_holder = encode_into_packets(full_list, erasure_count);

    let mut headered_packets: Vec<Vec<u8>> = vec![];
    for (_, ep) in packet_holder.1.into_iter().enumerate() {
        headered_packets.push(create_packet(packet_holder.0, batch_id, ep))
    }
    telemetry::debug!("Packets len {:?}", headered_packets.len());
    headered_packets
}

/// It takes a list of bytes and an erasure count, and returns a list of packets
///
/// Arguments:
///
/// * `unencoded_packet_list`: This is the list of packets that we want to
///   encode.
/// * `erasure_count`: The number of packets that can be lost and still be able
///   to recover the original
/// data.
///
/// Returns:
///
/// A vector of vectors of bytes.
pub fn encode_into_packets(
    unencoded_packet_list: &[u8],
    erasure_count: u32,
) -> (u16, Vec<Vec<u8>>) {
    let encoder = Encoder::with_defaults(unencoded_packet_list, (PAYLOAD_SIZE) as u16);
    println!("encoder :{:?}", encoder.get_config().symbol_size());

    let packets: Vec<Vec<u8>> = encoder
        .get_encoded_packets(erasure_count)
        .iter()
        .map(|packet| packet.serialize())
        .collect();
    (encoder.get_config().symbol_size(), packets)
}

/// It takes a packet and returns the batch id
///
/// Arguments:
///
/// * `packet`: The packet that we want to extract the batch id from.
///
/// Returns:
///
/// The batch_id is being returned.
// TODO: Make sure this is correct
// Seems like batch_id is of length = BATCH_ID_SIZE but is only overwritten at
// batch_id[0..BATCH_ID_SIZE - 3]. Last 3 elements will always be 0 here
#[allow(clippy::manual_memcpy)]
pub fn get_batch_id(packet: &[u8; 1280]) -> [u8; BATCH_ID_SIZE] {
    let mut batch_id: [u8; BATCH_ID_SIZE] = [0; BATCH_ID_SIZE];
    let mut chunk_no: usize = 0;
    for i in 10..(BATCH_ID_SIZE + 10) {
        batch_id[chunk_no] = packet[i];
        chunk_no += 1;
    }
    // The above equals to
    // batch_id[..(BATCH_ID_SIZE - 3)].copy_from_slice(&packet[6..(BATCH_ID_SIZE +
    // 3)]);
    batch_id
}

/// It takes a packet as an argument, and returns the length of the payload as
/// an integer
///
/// Arguments:
///
/// * `packet`: The packet that we want to get the payload length from.
///
/// Returns:
///
/// The length of the payload in bytes.
pub fn get_payload_length(packet: &[u8; 1280]) -> u32 {
    let mut payload_len_bytes: [u8; 4] = [0; 4];

    payload_len_bytes[..4].copy_from_slice(&packet[2..6]);
    u32::from_le_bytes(payload_len_bytes)
}

pub fn get_symbol_size(packet: &[u8; 1280]) -> u16 {
    let mut symbol_size_bytes: [u8; 2] = [0; 2];
    symbol_size_bytes[..2].copy_from_slice(&packet[6..8]);
    u16::from_le_bytes(symbol_size_bytes)
}

/// > Generate a random 32 byte batch id
pub fn generate_batch_id() -> [u8; BATCH_ID_SIZE] {
    let mut x = [0_u8; BATCH_ID_SIZE];
    thread_rng().fill_bytes(&mut x);
    let s: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(BATCH_ID_SIZE)
        .map(char::from)
        .collect();
    x.copy_from_slice(s.as_bytes());
    x
}

/// It reads the contents of a file into a byte array
///
/// Arguments:
///
/// * `file_path`: The path to the file you want to read.
pub fn read_file(file_path: PathBuf) -> Result<Vec<u8>> {
    let mut buffer;
    match fs::metadata(&file_path) {
        Ok(metadata) => {
            buffer = vec![0; metadata.len() as usize];
            if let Ok(mut file) = File::open(file_path) {
                match file.read_exact(&mut buffer) {
                    Ok(()) => {},
                    Err(e) => {
                        telemetry::error!("Error occured while reading a file, details {}", e);
                        return Err(e);
                    },
                }
            }
        },
        Err(e) => {
            telemetry::error!(
                "Error occured while reading metadata of file, details {}",
                e
            );
            return Err(e);
        },
    }
    Ok(buffer)
}

/// It receives a tuple of a string and a vector of bytes from a channel, and
/// writes the vector of bytes to a file whose name is the string
///
/// Arguments:
///
/// * `batch_recv`: Receiver<(String, Vec<u8>)>
pub fn batch_writer(batch_recv: Receiver<(String, Vec<u8>)>) {
    loop {
        match batch_recv.recv() {
            Ok((batch_id, contents)) => {
                let batch_fname = format!("{}.BATCH", batch_id);
                match fs::write(batch_fname, contents) {
                    Ok(_) => {},
                    Err(e) => {
                        telemetry::error!("Error occured while write data to file, details {}", e)
                    },
                }
            },
            Err(_e) => {
                continue;
            },
        };
    }
}

/// It receives packets from the `receiver` channel, checks if the packet is a
/// duplicate, and if not, it checks if the packet is a forwarder packet. If it
/// is, it forwards the packet to the `forwarder` channel. If it is not, it
/// checks if the packet is a new batch. If it is, it creates a new decoder
/// for the batch. If it is not, it adds the packet to the decoder. If the
/// decoder is complete, it sends the decoded file to the `file_send` channel
///
/// Arguments:
///
/// * `receiver`: Receiver<([u8; 1280], usize)>
/// * `batch_id_hashset`: A hashset that contains the batch_ids of all the
///   batches that have been
/// reassembled.
/// * `decoder_hash`: A hashmap that stores the batch_id as the key and a tuple
///   of the number of packets
/// received and the decoder as the value.
/// * `forwarder`: Sender<Vec<u8>>
/// * `file_send`: Sender<(String, Vec<u8>)>
pub fn reassemble_packets(
    receiver: Receiver<([u8; 1280], usize)>,
    batch_id_hashset: &mut HashSet<[u8; BATCH_ID_SIZE]>,
    decoder_hash: &mut HashMap<[u8; BATCH_ID_SIZE], (usize, Decoder)>,
    forwarder: Sender<Vec<u8>>,
    batch_send: Sender<RaptorBroadCastedData>,
) {
    loop {
        let mut received_packet = match receiver.recv() {
            Ok(pr) => pr,
            Err(_e) => {
                continue;
            },
        };

        let batch_id = get_batch_id(&received_packet.0);

        if batch_id_hashset.contains(&batch_id) {
            continue;
        }
        let payload_length = get_payload_length(&received_packet.0);
        let symbol_size = get_symbol_size(&received_packet.0);
        // This is to check if the packet is a forwarder packet. If it is, it forwards
        // the packet to the `forwarder` channel. Since packet is shared across
        // nodes with forward flag as 1
        if let Some(forward_flag) = received_packet.0.get_mut(1) {
            if *forward_flag == 1 {
                *forward_flag = 0;
                let _ = forwarder
                    .try_send(received_packet.0[DECODER_DATA_INDEX..received_packet.1].to_vec());
            }
        }

        match decoder_hash.get_mut(&batch_id) {
            Some((num_packets, decoder)) => {
                *num_packets += 1;
                // Decoding the packet.
                let result = decoder.decode(EncodingPacket::deserialize(
                    &received_packet.0[40_usize..received_packet.1],
                ));
                if result.is_some() {}
                if let Some(result_bytes) = result {
                    batch_id_hashset.insert(batch_id);
                    if let Ok(batch_id_str) = str::from_utf8(&batch_id) {
                        let batch_id_str = String::from(batch_id_str);
                        let msg = (batch_id_str, result_bytes);
                        if let Ok(data) = String::from_utf8(msg.1.clone()) {
                            let data = data.trim_end_matches('\0').to_string().replace("\\", "");
                            match serde_json::from_str::<RaptorBroadCastedData>(&data) {
                                Ok(data) => {
                                    let _ = batch_send.send(data);
                                },
                                Err(e) => {
                                    error!(
                                        "Error occured while unmarshalling  :{:?}",
                                        e.to_string()
                                    );
                                },
                            }
                        }
                        decoder_hash.remove(&batch_id);
                    }
                }
            },
            None => {
                // This is creating a new decoder for a new batch.
                decoder_hash.insert(
                    batch_id,
                    (
                        1_usize,
                        Decoder::new(ObjectTransmissionInformation::new(
                            payload_length as u64,
                            symbol_size as u16,
                            1,
                            1,
                            8,
                        )),
                    ),
                );
            },
        }
    }
}

//For Linux we can use system call from libc::recv_mmsg
/// It receives a UDP packet from a socket, and
/// returns the index of the packet in the array, the number of bytes received,
/// and the address of the sender
///
/// Arguments:
///
/// * `socket`: The UDP socket to receive from.
/// * `packets`: a mutable array of byte arrays, each of which is the size of
///   the largest packet you
/// want to receive.
//#[cfg(not(target_os = "linux"))]
pub async fn recv_mmsg(
    socket: &UdpSocket,
    packets: &mut [[u8; 1280]],
) -> Result<Vec<(usize, usize, SocketAddr)>> {
    let mut received = Vec::new();
    let count = std::cmp::min(NUM_RCVMMSGS, packets.len());
    for (i, packt) in packets.iter_mut().take(count).enumerate() {
        match socket.recv_from(packt).await {
            Err(e) => {
                return Err(e);
            },
            Ok((nrecv, from)) => {
                received.push((i, nrecv, from));
            },
        }
    }
    Ok(received)
}

/// It receives a packet from the `forwarder_channel_receive` channel, clones
/// it, and sends it to all the nodes in the network except itself
///
/// Arguments:
///
/// * `forwarder_channel_receive`: Receiver<Vec<u8>>
/// * `nodes_ips_except_self`: This is a vector of IP addresses of all the nodes
///   in the network except
/// the current node.
/// * `port`: The port to bind the socket to.
///
/// Returns:
///
/// A future that will be executed when the packet_forwarder function is called.
pub async fn packet_forwarder(
    forwarder_channel_receive: Receiver<Vec<u8>>,
    nodes_ips_except_self: Vec<Vec<u8>>,
    port: u16,
) -> Result<()> {
    if let Ok(sock) = UdpSocket::bind(SocketAddr::new(
        std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
        port,
    ))
    .await
    {
        let udp_socket = Arc::new(sock);

        loop {
            let nodes = nodes_ips_except_self.clone();
            match forwarder_channel_receive.recv() {
                Ok(packet) => {
                    println!("Received Forwarded packet ");
                    let mut broadcast_futures: Vec<_> = vec![];
                    for addr in nodes {
                        let pack = packet.clone();
                        let sock = udp_socket.clone();
                        broadcast_futures.push(tokio::task::spawn(async move {
                            sock.send_to(&pack, (&String::from_utf8_lossy(&addr)[..], port))
                                .await
                        }))
                    }
                    let _ = try_join_all(broadcast_futures).await;
                },
                Err(e) => {
                    telemetry::error!("Error occurred while receiving packet: {:?}", e)
                },
            }
        }
    } else {
        telemetry::error!("Error occured for binding port {} for udp socket", port);
        Ok(())
    }
}

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
    fn try_assemble(map: &mut Self::PacketMap) -> StdResult<Self::FlatPackets, NotCompleteError>;
}

/// Required to use `NotCompleteError` as an Error type in the Result enum
impl Error for NotCompleteError {}

/// Required to use `NotCompleteError` as an Error type in the Result enum
impl std::fmt::Display for NotCompleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "NotCompleteError")
    }
}
