use std::{collections::hash_map::DefaultHasher, net::SocketAddr, time::Duration};

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender};
use cuckoofilter::{CuckooFilter, ExportedCuckooFilter};
use events::{Event, EventMessage, EventPublisher, EventSubscriber, SyncPeerData};
use laminar::{Config, Packet, Socket, SocketEvent};
use network::{
    message::{Message, MessageBody},
    network::{BroadcastEngine, ConnectionIncoming},
};
use primitives::{
    ExportedFilter,
    NamespaceType,
    NodeType,
    NodeTypeBytes,
    PKShareBytes,
    PayloadBytes,
    QuorumPublicKey,
    RawSignature,
};
use serde::{Deserialize, Serialize};
use telemetry::{error, info, warn};

use crate::{NodeError, Result};


/// The number of erasures that the raptorq encoder will use to encode the
/// block.
const RAPTOR_ERASURE_COUNT: u32 = 3000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Data {
    Request(RendezvousRequest),
    Response(RendezvousResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendezvousRequest {
    Ping,
    Peers(Vec<u8>, ExportedFilter),
    Namespace(NodeTypeBytes, QuorumPublicKey),
    RegisterPeer(
        QuorumPublicKey,
        NodeTypeBytes,
        PKShareBytes,
        RawSignature,
        PayloadBytes,
        SyncPeerData,
    ),
    FetchNameSpace(NamespaceType),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendezvousResponse {
    Pong,
    RequestPeers(QuorumPublicKey),
    Peers(QuorumPublicKey, Vec<SyncPeerData>, ExportedFilter),
    PeerRegistered,
    NamespaceRegistered,
    Namespaces(Vec<QuorumPublicKey>),
}

#[derive(Debug)]
pub struct BroadcastEngineController {
    pub engine: BroadcastEngine,
    pub socket: Option<Socket>,
    pub quorum_key: QuorumPublicKey,
    pub rendezvous_local_addr: SocketAddr,
    pub rendezvous_server_addr: SocketAddr,
    events_tx: EventPublisher,
}

#[derive(Debug)]
pub struct BroadcastEngineControllerConfig {
    pub engine: BroadcastEngine,
    pub events_tx: EventPublisher,
}

impl BroadcastEngineControllerConfig {
    pub fn new(engine: BroadcastEngine, events_tx: EventPublisher) -> Self {
        Self { engine, events_tx }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.engine.local_addr()
    }
}

impl BroadcastEngineController {
    pub fn new(
        config: BroadcastEngineControllerConfig,
        rendezvous_local_addr: SocketAddr,
        rendezvous_server_addr: SocketAddr,
    ) -> Self {
        let engine = config.engine;
        let events_tx = config.events_tx;
        let socket_result = Socket::bind_with_config(
            rendezvous_local_addr,
            Config {
                blocking_mode: false,
                idle_connection_timeout: Duration::from_secs(5),
                heartbeat_interval: None,
                max_packet_size: (16 * 1024) as usize,
                max_fragments: 16_u8,
                fragment_size: 1024,
                fragment_reassembly_buffer_size: 64,
                receive_buffer_max_size: 1452_usize,
                rtt_smoothing_factor: 0.10,
                rtt_max_value: 250,
                socket_event_buffer_size: 1024,
                socket_polling_timeout: Some(Duration::from_millis(1000)),
                max_packets_in_flight: 512,
                max_unestablished_connections: 50,
            },
        );
        Self {
            engine,
            quorum_key: vec![],
            socket: socket_result.ok(),
            rendezvous_local_addr,
            rendezvous_server_addr,
            events_tx,
        }
    }

    pub async fn listen(
        &mut self,
        mut events_rx: EventSubscriber,
        sender_handle: Sender<()>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                Some((_conn, conn_incoming)) = self.engine.get_incoming_connections().next() => {
                match self.map_network_conn_to_message(conn_incoming).await {
                    Ok(message) => {
                        self.handle_network_event(message).await;
                    },
                     Err(err) => {
                        error!("unable to map connection into message: {err}");
                    }
                  }
                },
                Ok(event) = events_rx.recv() => {
                    if matches!(event.clone().into(), Event::Stop) {
                        sender_handle.send(());
                        info!("Stopping broadcast controller");
                        break
                    }
                    self.handle_internal_event(event.into()).await;
                },
            };
        }

        Ok(())
    }

    async fn handle_network_event(&mut self, message: Message) -> Result<()> {
        match message.data {
            MessageBody::InvalidBlock { .. } => {},
            MessageBody::Disconnect { .. } => {},
            MessageBody::StateComponents { .. } => {},
            MessageBody::Genesis { .. } => {},
            MessageBody::Child { .. } => {},
            MessageBody::Parent { .. } => {},
            MessageBody::Ledger { .. } => {},
            MessageBody::NetworkState { .. } => {},
            MessageBody::ClaimAbandoned { .. } => {},
            MessageBody::ResetPeerConnection { .. } => {},
            MessageBody::RemovePeer { .. } => {},
            MessageBody::AddPeer { .. } => {},
            MessageBody::DKGPartCommitment { .. } => {},
            MessageBody::DKGPartAcknowledgement { .. } => {},
            MessageBody::ForwardedTxn(txn_record) => {
                info!("Received Forwarded Txn :{:?}", txn_record.txn_id);
                let _ = self.events_tx.send(EventMessage::new(
                    None,
                    Event::NewTxnCreated(txn_record.txn),
                ));
            },
            MessageBody::Vote { .. } => {},
            MessageBody::Empty => {},
        };

        Ok(())
    }

    async fn handle_internal_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Stop => Ok(()),
            Event::QuorumKey(quorum_key) => {
                self.quorum_key = quorum_key;
                Ok(())
            },
            Event::PartMessage(sender_id, part_commitment) => {
                let status = self
                    .engine
                    .quic_broadcast(Message::new(MessageBody::DKGPartCommitment {
                        sender_id,
                        part_commitment,
                    }))
                    .await?;

                info!("Broadcasted part commitment to peers: {status:?}");
                Ok(())
            },
            Event::SyncPeers(quorum_key, peers, filter) => {
                if self.quorum_key == quorum_key {
                    if let Ok(restore_json) =
                        serde_json::from_slice::<ExportedCuckooFilter>(&filter)
                    {
                        let recovered_filter = CuckooFilter::<DefaultHasher>::from(restore_json);
                        if peers.is_empty() {
                            warn!("No peers to sync with");

                            self.events_tx.send(Event::EmptyPeerSync.into()).await?;

                            // TODO: revisit this return
                            return Ok(());
                        }

                        let mut quic_addresses = vec![];
                        let mut raptor_peer_list = vec![];
                        for peer in peers.iter() {
                            let addr = peer.address;

                            quic_addresses.push(addr);

                            let mut raptor_addr = addr;
                            raptor_addr.set_port(peer.raptor_udp_port);
                            raptor_peer_list.push(raptor_addr);
                        }
                        self.engine.add_raptor_peers(raptor_peer_list);

                        let peer_connection_result = self
                            .engine
                            .add_peer_connection(quic_addresses.clone())
                            .await;

                        if let Err(err) = peer_connection_result {
                            error!("unable to add peer connection: {err}");

                            self.events_tx
                                .send(Event::PeerSyncFailed(quic_addresses).into())
                                .await?;

                            return Err(err.into());
                        }

                        if let Ok(status) = peer_connection_result {
                            info!("{status:?}");
                        }

                        let mut drop_connection_list = Vec::new();
                        let list = self.engine.raptor_list.clone();
                        for peer in list.iter() {
                            if !recovered_filter.contains(&peer) {
                                self.engine.raptor_list.remove(&peer);
                                drop_connection_list.push(peer.clone());
                            }
                        }

                        self.engine.remove_peer_connection(drop_connection_list)?
                    }
                }

                Ok(())
            },
            Event::Vote(vote, farmer_quorum_threshold) => {
                let status = self
                    .engine
                    .quic_broadcast(Message::new(MessageBody::Vote {
                        vote,
                        farmer_quorum_threshold,
                    }))
                    .await?;

                info!("{status:?}");

                Ok(())
            },
            // Broadcasting the Convergence block to the peers.
            Event::BlockConfirmed(block) => {
                let status = self
                    .engine
                    .unreliable_broadcast(
                        block,
                        RAPTOR_ERASURE_COUNT,
                        self.engine.raptor_udp_port.clone(),
                    )
                    .await?;

                info!("{status:?}");

                Ok(())
            },
            Event::NamespaceRegistration(node_type, quorum_key) => {
                if let Some(socket) = self.socket.as_ref() {
                    let _ = self.send_namespace_registration(
                        &socket.get_packet_sender(),
                        node_type,
                        quorum_key,
                    );
                };
                Ok(())
            },
            Event::PeersFetch(quorum_key, filter) => {
                if let Some(socket) = self.socket.as_ref() {
                    let _ = self.send_retrieve_peers_request(
                        &socket.get_packet_sender(),
                        quorum_key,
                        filter,
                    );
                };
                Ok(())
            },
            Event::PeerRegistration(
                pk_share_bytes,
                quorum_key,
                msg_bytes,
                signature,
                node_type,
                quic_port,
            ) => {
                if let Some(socket) = self.socket.as_ref() {
                    let _ = self.send_register_peer_payload(
                        &socket.get_packet_sender(),
                        quorum_key,
                        pk_share_bytes,
                        msg_bytes,
                        signature,
                        node_type,
                        quic_port,
                    );
                };

                Ok(())
            },
            Event::PullFarmerNamespaces => {
                if let Some(socket) = self.socket.as_ref() {
                    let _ = self.pull_namespaces(
                        &socket.get_packet_sender(),
                        "FARMER".to_string().into_bytes(),
                    );
                };
                Ok(())
            },
            Event::PullHarvesterNamespaces => {
                if let Some(socket) = self.socket.as_ref() {
                    let _ = self.pull_namespaces(
                        &socket.get_packet_sender(),
                        "FARMER".to_string().into_bytes(),
                    );
                };
                Ok(())
            },
            _ => Ok(()),
        }
    }

    /// Turns connection data into Message then returns it
    async fn map_network_conn_to_message(
        &self,
        mut conn_incoming: ConnectionIncoming,
    ) -> Result<Message> {
        let res = conn_incoming.next().await.map_err(|err| {
            NodeError::Other(format!("unable to listen for new connections: {err}"))
        })?;

        let (_, _, raw_message) = res.unwrap_or((Bytes::new(), Bytes::new(), Bytes::new()));
        let message = Message::from(raw_message.to_vec());

        Ok(message)
    }

    pub fn process_rendezvous_response(&self, receiver_handle: Receiver<()>) -> Result<()> {
        let socket = self.socket.as_ref().ok_or_else(|| {
            NodeError::Other(String::from(
                "Failed to obtain socket; it may happen socket is not initialized",
            ))
        })?;
        let receiver = socket.get_event_receiver();
        let sender = socket.get_packet_sender();
        loop {
            if let Ok(event) = receiver.recv_timeout(Duration::from_millis(100)) {
                self.process_rendezvous_event(&event, &sender);
            }
            if let Ok(()) = receiver_handle.recv() {
                return Ok(());
            }
        }
    }

    fn process_rendezvous_event(&self, event: &SocketEvent, sender: &Sender<Packet>) {
        match event {
            SocketEvent::Packet(packet) => self.process_packet(packet, sender),
            SocketEvent::Timeout(_) => {},
            _ => {},
        }
    }

    fn process_packet(&self, packet: &Packet, sender: &Sender<Packet>) {
        if packet.addr() == self.rendezvous_server_addr {
            if let Ok(payload_response) = bincode::deserialize::<Data>(packet.payload()) {
                self.process_payload_response(&payload_response, sender, packet);
            }
        }
    }

    fn process_payload_response(
        &self,
        payload_response: &Data,
        sender: &Sender<Packet>,
        packet: &Packet,
    ) {
        match payload_response {
            Data::Request(req) => self.process_request(req, sender, packet),
            Data::Response(resp) => self.process_response(resp),
        }
    }

    fn process_request(
        &self,
        request: &RendezvousRequest,
        sender: &Sender<Packet>,
        packet: &Packet,
    ) {
        match request {
            RendezvousRequest::Ping => {
                let response = &Data::Response(RendezvousResponse::Pong);
                if let Ok(data) = bincode::serialize(&response) {
                    let _ = sender.send(Packet::reliable_unordered(packet.addr(), data));
                }
            },
            _ => {},
        }
    }

    fn process_response(&self, response: &RendezvousResponse) {
        match response {
            RendezvousResponse::Peers(quorum_key, peers, filter) => {
                let _ = self.events_tx.send(
                    Event::SyncPeers(quorum_key.clone(), peers.clone(), filter.clone()).into(),
                );
            },
            RendezvousResponse::NamespaceRegistered => {
                info!("Namespace Registered");
            },
            RendezvousResponse::PeerRegistered => {
                info!("Peer Registered");
            },
            RendezvousResponse::Namespaces(namespaces) => {
                let _ = self
                    .events_tx
                    .send(Event::UpdateFarmerNamespaces(namespaces.clone()).into());
            },
            _ => {},
        }
    }

    fn send_namespace_registration(
        &self,
        sender: &Sender<Packet>,
        node_type: NodeType,
        quorum_key: QuorumPublicKey,
    ) {
        if let Ok(data) = bincode::serialize(&Data::Request(RendezvousRequest::Namespace(
            node_type.to_string().as_bytes().to_vec(),
            quorum_key,
        ))) {
            let _ = sender.send(Packet::reliable_ordered(
                self.rendezvous_server_addr,
                data,
                None,
            ));
        }
    }

    fn send_retrieve_peers_request(
        &self,
        sender: &Sender<Packet>,
        quorum_key: QuorumPublicKey,
        filter: Vec<u8>,
    ) {
        if let Ok(data) =
            bincode::serialize(&Data::Request(RendezvousRequest::Peers(quorum_key, filter)))
        {
            let _ = sender.send(Packet::reliable_ordered(
                self.rendezvous_server_addr,
                data,
                None,
            ));
        }
    }

    fn send_register_peer_payload(
        &self,
        sender: &Sender<Packet>,
        quorum_key: QuorumPublicKey,
        pk_share_bytes: PKShareBytes,
        msg_bytes: Vec<u8>,
        signature: Vec<u8>,
        node_type: NodeType,
        quic_port: u16,
    ) {
        let payload_result = bincode::serialize(&Data::Request(RendezvousRequest::RegisterPeer(
            quorum_key,
            node_type.to_string().into_bytes(),
            pk_share_bytes,
            signature,
            msg_bytes,
            SyncPeerData {
                address: self.rendezvous_local_addr,
                raptor_udp_port: self.rendezvous_local_addr.port(),
                quic_port,
                node_type,
            },
        )));
        if let Ok(payload) = payload_result {
            let _ = sender.send(Packet::reliable_ordered(
                self.rendezvous_server_addr,
                payload,
                None,
            ));
        }
    }

    fn pull_namespaces(&self, sender: &Sender<Packet>, namespace: Vec<u8>) {
        let payload_result =
            bincode::serialize(&Data::Request(RendezvousRequest::FetchNameSpace(namespace)));
        if let Ok(payload) = payload_result {
            let _ = sender.send(Packet::reliable_ordered(
                self.rendezvous_server_addr,
                payload,
                None,
            ));
        }
    }
}
