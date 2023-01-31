use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    net::{Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use std::net::{IpAddr, Ipv4Addr};

use bytes::Bytes;
use crossbeam_channel::{Sender, unbounded};
use futures::{stream::FuturesUnordered, StreamExt};
use qp2p::{
    Config,
    Connection,
    ConnectionIncoming,
    Endpoint,
    EndpointError,
    IncomingConnections,
    RetryConfig,
};
use raptorq::Decoder;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use telemetry::{info, tracing};

use crate::{
    message::Message,
    packet::{
        generate_batch_id,
        packet_forwarder,
        reassemble_packets,
        recv_mmsg,
        split_into_packets,
        BATCH_ID_SIZE,
        MTU_SIZE,
        NUM_RCVMMSGS,
    },
    types::config::{BroadCastError, BroadCastResult},
};
use crate::packet::RaptorBroadCastedData;

type BroadCastStatus = Result<BroadCastResult, BroadCastError>;

/// This is an enum that is used to determine the type of broadcast that is
/// being used.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BroadcastType {
    Quic,
    ReliableUDP,
    UnReliableUDP,
}

#[derive(Debug)]
pub struct BroadcastEngine {
    pub peer_connection_list: Arc<Mutex<Vec<(SocketAddr, Connection)>>>,
    pub raptor_list: Arc<Mutex<Vec<SocketAddr>>>,
    pub endpoint: (Endpoint, IncomingConnections),
    pub raptor_udp_port: u16,
    pub raptor_num_packet_blast: usize,
}

const CONNECTION_CLOSED: &str = "The connection was closed intentionally by qp2p.";

pub type EndPointResult = Result<
    (
        Endpoint,
        IncomingConnections,
        Option<(Connection, ConnectionIncoming)>,
    ),
    EndpointError,
>;

impl BroadcastEngine {
    pub async fn new_endpoint(port: u16) -> EndPointResult {
        Endpoint::new_peer(
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
        .await
    }

    //create a new broadcast engine for each node
    pub async fn new(
        raptor_udp_port: u16,
        raptor_num_packet_blast: usize,
    ) -> Result<BroadcastEngine, BroadCastError> {
        match BroadcastEngine::new_endpoint(raptor_udp_port).await {
            Ok((node, incoming_conns, _contact)) => Ok(BroadcastEngine {
                peer_connection_list: Arc::new(Mutex::new(Vec::new())),
                raptor_list: Arc::new(Mutex::new(vec![])),
                endpoint: (node, incoming_conns),
                raptor_udp_port,
                raptor_num_packet_blast,
            }),
            Err(e) => Err(BroadCastError::EndpointError(e)),
        }
    }

    /// > This function takes a vector of socket addresses and attempts to
    /// > connect to each one. If the
    /// connection is successful, it adds the connection to the peer connection
    /// list
    ///
    /// Arguments:
    ///
    /// * `address`: A vector of SocketAddr, which is the address of the peer
    ///   you want to connect to.

    // TODO: Rething this - as the Mutex is locked during the `.await`, which is not
    // reccomended. Either async-aware Mutex type should be used, or MutexGuard
    // should be dropped before `.await`
    #[allow(clippy::await_holding_lock)]
    pub async fn add_peer_connection(&mut self, address: Vec<SocketAddr>) -> BroadCastStatus {
        if let Ok(mut peers) = self.peer_connection_list.lock() {
            for addr in address.iter() {
                let result = self.endpoint.0.connect_to(addr).await;
                match result {
                    Ok((connection, _)) => {
                        peers.push((*addr, connection));
                    },
                    Err(e) => {
                        return Err(BroadCastError::ConnectionError(e));
                    },
                }
            }
            std::mem::drop(peers);
        } else {
            telemetry::error!("Error acquiring lock on peer connection list");
        }
        Ok(BroadCastResult::ConnectionEstablished)
    }


    pub async fn add_raptor_peers(&mut self, address: Vec<SocketAddr>) -> BroadCastStatus {
        if let Ok(mut peers) = self.raptor_list.lock() {
            peers.extend(address);
        } else {
            telemetry::error!("Error acquiring lock on peer connection list");
        }
        Ok(BroadCastResult::ConnectionEstablished)
    }

    /// This function removes a peer connection from the peer connection list
    ///
    /// Arguments:
    ///
    /// * `address`: The address of the peer to be removed.
    ///
    /// Returns:
    ///
    /// A boolean value.
    pub fn remove_peer_connection(&mut self, address: Vec<SocketAddr>) -> BroadCastStatus {
        if let Ok(mut peers) = self.peer_connection_list.lock() {
            for addr in address.iter() {
                peers.retain(|address| {
                    address.1.close(Some(String::from(CONNECTION_CLOSED)));
                    address.0 != *addr
                });
            }
            return Ok(BroadCastResult::Success);
        } else {
            telemetry::error!("Error acquiring lock on peer connection list");
        };

        Ok(BroadCastResult::Success)
    }

    /// > This function takes a message and sends it to all the peers in the
    /// > peer list
    ///
    /// Arguments:
    ///
    /// * `message`: Message - The message to be broadcasted
    ///
    /// Returns:
    ///
    /// A future that resolves to a BroadCastStatus
    // TODO: Again - the Mutex is held during .await - to be reconsidered
    #[allow(clippy::await_holding_lock)]
    pub async fn quic_broadcast(&self, message: Message) -> BroadCastStatus {
        let mut futs = FuturesUnordered::new();
        if let Ok(peers) = self.peer_connection_list.lock() {
            if peers.len() == 0 {
                return Err(BroadCastError::NoPeers);
            }
            for connection in peers.clone().into_iter() {
                let new_data = message.as_bytes().clone();
                futs.push(tokio::spawn(async move {
                    let msg = Bytes::from(new_data);
                    let status = connection
                        .1
                        .send((Bytes::new(), Bytes::new(), msg.clone()))
                        .await;
                    if let Err(err) = status {
                        return Err(BroadCastError::BroadcastingDataError(err));
                    }
                    Ok(())
                }))
            }
            while futs.next().await.is_some() {}
        } else {
            telemetry::error!("Error acquiring lock on peer connection list");
        }
        Ok(BroadCastResult::Success)
    }

    /// > This function takes a message and an address, and sends the message to
    /// > the address via QUIC
    ///
    /// Arguments:
    ///
    /// * `message`: Message - The message to be sent
    /// * `addr`: The address of the node to which we want to send the message.
    ///
    /// Returns:
    ///
    /// A future that resolves to a BroadCastStatus
    pub async fn send_data_via_quic(&self, message: Message, addr: SocketAddr) -> BroadCastStatus {
        let msg = Bytes::from(message.as_bytes());
        let node = self.endpoint.0.clone();
        let result = node.connect_to(&addr).await;
        let conn = match result {
            Ok(conn) => conn.0,
            Err(e) => return Err(BroadCastError::ConnectionError(e)),
        };
        let _ = conn.send((Bytes::new(), Bytes::new(), msg.clone())).await;
        Ok(BroadCastResult::Success)
    }

    /// > The function takes a message and an erasure count as input and splits
    /// > the message into packets
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
    /// The return type is a future of type BroadCastStatus.
    // TODO: Verify if mutex really needs to be held during the await
    #[allow(clippy::await_holding_lock)]

    pub async fn unreliable_broadcast(
        &self,
        data:Vec<u8>,
        erasure_count: u32,
        port:u16
    ) -> BroadCastStatus {
        println!("Broadcasting to Port {:?}", port);
        let batch_id = generate_batch_id();
        let chunks = split_into_packets(&data, batch_id, erasure_count);
        if let Ok(udp_socket) = UdpSocket::bind(SocketAddr::new(
            std::net::IpAddr ::V4(Ipv4Addr::new(127,0,0,1)),
            port
        ))
        .await
        {
            let udp_socket = Arc::new(udp_socket);
            let mut futs = FuturesUnordered::new();
            if let Ok(peers) = self.raptor_list.lock() {
                if peers.len() == 0 {
                    return Err(BroadCastError::NoPeers);
                }
                for (packet_index, packet) in chunks.iter().enumerate() {
                    // Sharding/Distribution of packets as per no of nodes
                    let address: SocketAddr =
                        peers.get(packet_index % peers.len()).unwrap().clone();
                    let packet = packet.clone();
                    let sock = udp_socket.clone();

                    futs.push(tokio::spawn(async move {
                        let addr = address.to_string();
                        let s = sock.send_to(&packet, addr.clone()).await;
                    }));

                    if futs.len() >= self.raptor_num_packet_blast {
                        match futs.next().await {
                            Some(fut) => {
                                if fut.is_err() {
                                    telemetry::error!("Sending future is not ready yet")
                                }
                            },
                            None => telemetry::error!("Sending future is not ready yet"),
                        }
                    }
                }

                while (futs.next().await).is_some() {}
            } else {
                telemetry::error!("Error acquiring lock on peer connection list");
            }
        } else {
            telemetry::error!("Error occured while binding socket to a port.");
        }

        Ok(BroadCastResult::Success)
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
    pub async fn process_received_packets(&self, port: u16, batch_sender: Sender<RaptorBroadCastedData>) -> Result<(), BroadCastError> {

        if let Ok(sock_recv) = UdpSocket::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        ))
        .await{
            info!("Listening on {}", port);
            let buf = [0; MTU_SIZE];
            let (reassembler_channel_send, reassembler_channel_receive) = unbounded();
            let (forwarder_send, forwarder_receive) = unbounded();
            let mut batch_id_store: HashSet<[u8; BATCH_ID_SIZE]> = HashSet::new();
            let mut decoder_hash: HashMap<[u8; BATCH_ID_SIZE], (usize, Decoder)> = HashMap::new();

            thread::spawn({
                let assemble_send = reassembler_channel_send.clone();
                let fwd_send = forwarder_send.clone();
                let batch_send = batch_sender.clone();

                move || {
                    reassemble_packets(
                        reassembler_channel_receive,
                        &mut batch_id_store,
                        &mut decoder_hash,
                        fwd_send.clone(),
                        batch_send.clone(),
                    );
                    drop(assemble_send);
                    drop(fwd_send);
                    drop(batch_send);
                }
            });

            let mut nodes_ips_except_self = vec![];
            if let Ok(peers) = self.raptor_list.lock() {
                if peers.len() == 0 {
                    return Err(BroadCastError::NoPeers);
                }
                peers.iter().for_each(|(addr)| {
                    nodes_ips_except_self.push(addr.to_string().as_bytes().to_vec())
                });
            } else {
                telemetry::error!("Error acquiring lock on peer connection list");
            }

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
        } else {
            telemetry::error!("Udp port {} already in use", port);
            Err(BroadCastError::EaddrInUse)
        }
    }

    pub fn get_incomming_connections(&mut self) -> &mut IncomingConnections {
        &mut self.endpoint.1
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.endpoint.0.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv6Addr, SocketAddr},
        time::Duration,
    };

    use bytes::Bytes;

    use crate::{message::Message, network::BroadcastEngine};

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
            if let Some((_, incoming)) = b2.get_incomming_connections().next().await {
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
            if let Some((_, incoming)) = b3.get_incomming_connections().next().await {
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
            data: "Hello_VRRB".to_string().as_bytes().to_vec(),
            source: Some("vrrb".to_string().as_bytes().to_vec()),
            sequence_number: Some(1i32.to_ne_bytes().to_vec()),
            return_receipt: 0u8,
        };
        msg
    }

    trait Timeout: Sized {
        fn timeout(self) -> tokio::time::Timeout<Self>;
    }

    impl<F: std::future::Future> Timeout for F {
        fn timeout(self) -> tokio::time::Timeout<Self> {
            tokio::time::timeout(Duration::from_secs(5), self)
        }
    }
}
