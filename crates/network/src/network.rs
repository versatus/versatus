use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    thread,
    time::Duration,
};

use bytes::Bytes;
use crossbeam_channel::{unbounded, Sender};
use futures::{stream::FuturesUnordered, StreamExt};
use primitives::{
    DEFAULT_CONNECTION_TIMEOUT_IN_SECS,
    RAPTOR_DECODER_CACHE_LIMIT,
    RAPTOR_DECODER_CACHE_TTL_IN_SECS,
};
use qp2p::ConnectionError;
pub use qp2p::{
    Config,
    Connection,
    ConnectionIncoming,
    Endpoint,
    IncomingConnections,
    RetryConfig,
};
use raptorq::Decoder;
use serde::{Deserialize, Serialize};
use telemetry::{error, info};
use tokio::net::UdpSocket;
use vrrb_core::cache::Cache;

use crate::{
    config::BroadcastError,
    message::Message,
    packet::{
        generate_batch_id,
        packet_forwarder,
        reassemble_packets,
        recv_mmsg,
        split_into_packets,
        RaptorBroadCastedData,
        BATCH_ID_SIZE,
        MTU_SIZE,
        NUM_RCVMMSGS,
    },
    types::config::BroadcastStatus,
};

pub type Result<T> = std::result::Result<T, BroadcastError>;

/// This is an enum that is used to determine the type of broadcast that is
/// being used.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BroadcastType {
    Quic,
    ReliableUDP,
    UnreliableUDP,
}
trait Timeout: Sized {
    fn timeout(self) -> tokio::time::Timeout<Self>;
}

impl<F: std::future::Future> Timeout for F {
    fn timeout(self) -> tokio::time::Timeout<Self> {
        tokio::time::timeout(
            Duration::from_secs(DEFAULT_CONNECTION_TIMEOUT_IN_SECS),
            self,
        )
    }
}
#[derive(Debug)]
pub struct BroadcastEngine {
    pub peer_connection_list: HashMap<SocketAddr, Connection>,
    pub raptor_list: HashSet<SocketAddr>,
    pub endpoint: (Endpoint, IncomingConnections),
    pub raptor_udp_port: u16,
    pub raptor_num_packet_blast: usize,
}

const CONNECTION_CLOSED: &str = "The connection was closed intentionally by qp2p.";

impl BroadcastEngine {
    /// Create a new broadcast engine for each node
    #[telemetry::instrument]
    pub async fn new(raptor_udp_port: u16, raptor_num_packet_blast: usize) -> Result<Self> {
        let (node, incoming_conns, _) = Self::new_endpoint(raptor_udp_port).await?;

        Ok(Self {
            peer_connection_list: HashMap::new(),
            raptor_list: HashSet::new(),
            endpoint: (node, incoming_conns),
            raptor_udp_port,
            raptor_num_packet_blast,
        })
    }

    #[telemetry::instrument]
    async fn new_endpoint(
        port: u16,
    ) -> Result<(
        Endpoint,
        IncomingConnections,
        Option<(Connection, ConnectionIncoming)>,
    )> {
        let (endpoint, incoming_connections, conn_opts) = Endpoint::new_peer(
            (Ipv6Addr::LOCALHOST, port),
            &[],
            Config {
                retry_config: RetryConfig {
                    retrying_max_elapsed_time: Duration::from_millis(500),
                    ..RetryConfig::default()
                },
                keep_alive_interval: Some(Duration::from_secs(5)),
                ..Config::default()
            },
        )
        .await?;

        Ok((endpoint, incoming_connections, conn_opts))
    }

    /// This function takes a vector of socket addresses and attempts to
    /// connect to each one. If the
    /// connection is successful, it adds the connection to the peer connection
    /// list
    ///
    /// Arguments:
    ///
    /// * `address`: A vector of SocketAddr, which is the address of the peer
    ///   you want to connect to.
    #[telemetry::instrument]
    pub async fn add_peer_connection(
        &mut self,
        address: Vec<SocketAddr>,
    ) -> Result<BroadcastStatus> {
        for addr in address.iter() {
            let connection_result = self.endpoint.0.connect_to(addr).timeout().await;
            match connection_result {
                Ok(con_result) => {
                    let (connection, _) = con_result.map_err(|err| {
                        error!("failed to connect with {addr}: {err}");
                        BroadcastError::Connection(err)
                    })?;
                    self.peer_connection_list
                        .insert(addr.to_owned(), connection);
                },
                Err(e) => {
                    error!("Connection error  {addr}: {e}");
                },
            }
        }
        return Ok(BroadcastStatus::Success);
    }

    #[telemetry::instrument]
    pub fn add_raptor_peers(&mut self, address: Vec<SocketAddr>) {
        self.raptor_list.extend(address)
    }

    /// This function removes a peer connection from the peer connection list
    ///
    /// Arguments:
    ///
    /// * `address`: The address of the peer to be removed.
    #[telemetry::instrument]
    pub fn remove_peer_connection(&mut self, address: Vec<SocketAddr>) -> Result<()> {
        for addr in address.iter() {
            self.peer_connection_list.retain(|address, connection| {
                info!("closed connection with: {addr}");
                connection.close(Some(String::from(CONNECTION_CLOSED)));
                address != addr
            });
        }

        Ok(())
    }

    /// This function takes a message and sends it to all the peers in the
    /// peer list
    ///
    /// Arguments:
    ///
    /// * `message`: Message - The message to be broadcasted
    ///
    /// Returns:
    ///
    /// A future that resolves to a BroadcastStatus
    #[telemetry::instrument(name = "quic_broadcast")]
    pub async fn quic_broadcast(&self, message: Message) -> Result<BroadcastStatus> {
        let mut futs = FuturesUnordered::new();

        if self.peer_connection_list.is_empty() {
            return Err(BroadcastError::NoPeers);
        }

        for (addr, conn) in self.peer_connection_list.clone().into_iter() {
            let new_data = message.as_bytes().clone();

            futs.push(tokio::spawn(async move {
                let msg = Bytes::from(new_data);

                match conn.send((Bytes::new(), Bytes::new(), msg.clone())).await {
                    Ok(_) => {
                        info!("sent message to {addr}");
                    },
                    Err(err) => {
                        error!("send error: {err}");
                    },
                }
            }))
        }

        while futs.next().await.is_some() {}

        Ok(BroadcastStatus::Success)
    }

    /// This function takes a message and an address, and sends the message to
    /// the address via QUIC
    ///
    /// Arguments:
    ///
    /// * `message`: Message - The message to be sent
    /// * `addr`: The address of the node to which we want to send the message.
    ///
    /// Returns:
    ///
    /// A future that resolves to a BroadcastStatus
    #[telemetry::instrument(name = "send_data_via_quic")]
    pub async fn send_data_via_quic(
        &self,
        message: Message,
        addr: SocketAddr,
    ) -> Result<BroadcastStatus> {
        let msg = Bytes::from(message.as_bytes());
        let node = self.endpoint.0.clone();
        let conn_result = node.connect_to(&addr).timeout().await;
        match conn_result {
            Ok(conn) => {
                let conn = conn?.0;
                let _ = conn.send((Bytes::new(), Bytes::new(), msg.clone())).await;
                return Ok(BroadcastStatus::Success);
            },
            Err(e) => {
                error!("Connection error  {addr}: {e}");
                return Err(BroadcastError::Other(e.to_string()));
            },
        }
    }

    /// The function takes a message and an erasure count as input and splits
    /// the message into packets
    /// and sends them to the peers
    ///
    /// Arguments:
    ///
    /// * `message`: The message to be broadcasted.
    /// * `erasure_count`: The number of packets that can be lost and still be
    ///   able to reconstruct the
    /// original message.
    ///
    /// Returns:
    ///
    /// The return type is a future of type BroadcastStatus.
    #[telemetry::instrument(name = "unreliable_broadcast")]
    pub async fn unreliable_broadcast(
        &self,
        data: Vec<u8>,
        erasure_count: u32,
        port: u16,
    ) -> Result<BroadcastStatus> {
        info!("broadcasting to port {:?}", port);

        let batch_id = generate_batch_id();
        let chunks = split_into_packets(&data, batch_id, erasure_count);

        let ipv4_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let udp_socket = UdpSocket::bind(SocketAddr::new(ipv4_addr, port))
            .await
            .map_err(|err| BroadcastError::Other(err.to_string()))?;

        let udp_socket = Arc::new(udp_socket);
        let mut futs = FuturesUnordered::new();

        if self.raptor_list.is_empty() {
            return Err(BroadcastError::NoPeers);
        }

        for (packet_index, packet) in chunks.iter().enumerate() {
            let addresses =
                self.get_address_for_packet_shards(packet_index, self.raptor_list.len());

            for address in addresses.into_iter() {
                let packet = packet.clone();
                let sock = udp_socket.clone();
                futs.push(tokio::spawn(async move {
                    let addr = address.to_string();
                    let _ = sock.send_to(&packet, addr.clone()).await;
                }));
                if futs.len() >= self.raptor_num_packet_blast {
                    match futs.next().await {
                        Some(fut) => {
                            if fut.is_err() {
                                error!("Sending future is not ready yet")
                            }
                        },
                        None => error!("Sending future is not ready yet"),
                    }
                }
            }
        }

        while (futs.next().await).is_some() {}

        Ok(BroadcastStatus::Success)
    }

    /// It receives packets from the socket, and sends them to the reassembler
    /// thread
    ///
    /// Arguments:
    ///
    /// * `port`: The port on which the node is listening for incoming packets.
    ///
    /// Returns:
    ///
    /// a future that resolves to a result. The result is either an error or a
    /// unit.
    #[telemetry::instrument(name = "process_received_packets")]
    pub async fn process_received_packets(
        &self,
        port: u16,
        batch_sender: Sender<RaptorBroadCastedData>,
    ) -> Result<()> {
        let sock_recv = UdpSocket::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        ))
        .await
        .map_err(|err| {
            error!("UDP port {port} already in use");
            BroadcastError::Other(err.to_string())
        })?;

        info!("Listening on {}", port);

        let buf = [0; MTU_SIZE];
        let (reassembler_channel_send, reassembler_channel_receive) = unbounded();
        let (forwarder_send, forwarder_receive) = unbounded();
        let mut batch_id_store: HashSet<[u8; BATCH_ID_SIZE]> = HashSet::new();

        let mut decoder_hash_cache: Cache<[u8; BATCH_ID_SIZE], (usize, Decoder)> =
            Cache::new(RAPTOR_DECODER_CACHE_LIMIT, RAPTOR_DECODER_CACHE_TTL_IN_SECS);

        thread::spawn({
            let assemble_send = reassembler_channel_send.clone();
            let fwd_send = forwarder_send.clone();
            let batch_send = batch_sender.clone();

            move || {
                reassemble_packets(
                    reassembler_channel_receive,
                    &mut batch_id_store,
                    &mut decoder_hash_cache,
                    fwd_send.clone(),
                    batch_send.clone(),
                );
                drop(assemble_send);
                drop(fwd_send);
                drop(batch_send);
            }
        });

        let mut nodes_ips_except_self = vec![];
        if self.raptor_list.is_empty() {
            return Err(BroadcastError::NoPeers);
        }

        self.raptor_list
            .iter()
            .for_each(|addr| nodes_ips_except_self.push(addr.to_string().as_bytes().to_vec()));

        let port = self.raptor_udp_port;
        thread::spawn(move || packet_forwarder(forwarder_receive, nodes_ips_except_self, port));

        loop {
            let mut receive_buffers = [buf; NUM_RCVMMSGS];
            // Receiving a batch of packets from the socket.
            if let Ok(res) = recv_mmsg(&sock_recv, receive_buffers.borrow_mut()).await {
                if !res.is_empty() {
                    let mut i = 0;
                    for buf in &receive_buffers {
                        if let Some(packets_info) = res.get(i) {
                            let _ = reassembler_channel_send.send((*buf, packets_info.1));
                            i += 1;
                        }
                    }
                }
            }
        }
    }

    #[telemetry::instrument(name = "get_incoming_connections")]
    pub fn get_incoming_connections(&mut self) -> &mut IncomingConnections {
        &mut self.endpoint.1
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.endpoint.0.local_addr()
    }

    fn get_address_for_packet_shards(
        &self,
        packet_index: usize,
        total_peers: usize,
    ) -> Vec<SocketAddr> {
        let mut addresses = Vec::new();
        let number_of_peers = (total_peers as f32 * 0.10).ceil() as usize;
        let raptor_list_cloned: Vec<&SocketAddr> = self.raptor_list.iter().collect();

        for i in 0..number_of_peers {
            if let Some(address) = raptor_list_cloned.get(packet_index % (total_peers + i)) {
                // TODO: refactor this double owning
                addresses.push(address.to_owned().to_owned());
            }
        }

        addresses
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddr};

    use bytes::Bytes;

    use crate::{
        message::{Message, MessageBody},
        network::{BroadcastEngine, Timeout},
    };

    #[tokio::test]
    async fn test_successful_connection() {
        let mut b1 = BroadcastEngine::new(1234, 1145).await.unwrap();
        let mut b2 = BroadcastEngine::new(1235, 1145).await.unwrap();

        let _ = b1
            .add_peer_connection(vec![SocketAddr::new(
                std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                1235,
            )])
            .await;

        let _ = b2
            .add_peer_connection(vec![SocketAddr::new(
                std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                1234,
            )])
            .await;

        if let Some((connection, _)) = b1.endpoint.1.next().await {
            assert_eq!(connection.remote_address(), b2.endpoint.0.public_addr());
        } else {
            panic!("No incoming connection");
        }

        let _ = b1.remove_peer_connection(vec![SocketAddr::new(
            std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            1234,
        )]);
        let _ = b1.remove_peer_connection(vec![SocketAddr::new(
            std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            1235,
        )]);
    }

    #[tokio::test]
    async fn test_broadcast_message_to_peers() {
        let mut b1 = BroadcastEngine::new(1236, 1145).await.unwrap();
        let mut b2 = BroadcastEngine::new(1237, 1145).await.unwrap();
        let mut b3 = BroadcastEngine::new(1238, 1145).await.unwrap();

        let _ = b1
            .add_peer_connection(vec![SocketAddr::new(
                std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                1237,
            )])
            .await;

        let _ = b1
            .add_peer_connection(vec![SocketAddr::new(
                std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                1238,
            )])
            .await;

        let tst_msg = test_message();
        let _ = b1.quic_broadcast(tst_msg.clone()).await;

        let _ = b1.quic_broadcast(tst_msg.clone()).await;

        // Peer 2 gets an incoming connection
        let mut peer2_incoming_messages =
            if let Some((_, incoming)) = b2.get_incoming_connections().next().await {
                incoming
            } else {
                panic!("No incoming connection");
            };

        if let Ok(message) = peer2_incoming_messages.next().timeout().await.unwrap() {
            assert_eq!(
                message,
                Some((Bytes::new(), Bytes::new(), Bytes::from(tst_msg.as_bytes())))
            );
        }

        // Peer 2 gets an incoming connection
        let mut peer3_incoming_messages =
            if let Some((_, incoming)) = b3.get_incoming_connections().next().await {
                incoming
            } else {
                panic!("No incoming connection");
            };

        if let Ok(message) = peer3_incoming_messages.next().timeout().await.unwrap() {
            assert_eq!(
                message,
                Some((Bytes::new(), Bytes::new(), Bytes::from(tst_msg.as_bytes())))
            );
        }
    }

    pub fn test_message() -> Message {
        let msg = Message {
            id: uuid::Uuid::new_v4(),
            data: MessageBody::Empty,
        };
        msg
    }
}
