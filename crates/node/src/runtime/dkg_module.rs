use std::{
    net::{SocketAddr, Ipv4Addr, IpAddr},
    thread,
    thread::sleep,
    time::Duration,
};

use async_trait::async_trait;
use crossbeam_channel::{select, unbounded, Sender};
use dkg_engine::{
    dkg::DkgGenerator,
    types::{config::ThresholdConfig, DkgEngine, DkgResult},
};
use events::{Event, SyncPeerData};
use hbbft::crypto::{PublicKey, SecretKeyShare};
use laminar::{Config, Packet, Socket, SocketEvent};
use primitives::{
    NodeIdx,
    NodeType,
    NodeTypeBytes,
    PKShareBytes,
    PayloadBytes,
    QuorumPublicKey,
    QuorumType,
    RawSignature,
    REGISTER_REQUEST,
    RETRIEVE_PEERS_REQUEST,
};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tracing::error;

use crate::{result::Result, NodeError};

pub struct DkgModuleConfig {
    pub quorum_type: Option<QuorumType>,
    pub quorum_size: usize,
    pub quorum_threshold: usize,
}

pub struct DkgModule {
    pub dkg_engine: DkgEngine,
    pub quorum_type: Option<QuorumType>,
    pub rendezvous_local_addr: SocketAddr,
    pub rendezvous_server_addr: SocketAddr,
    pub quic_port: u16,
    pub socket: Socket,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl DkgModule {
    pub fn new(
        node_idx: NodeIdx,
        node_type: NodeType,
        secret_key: hbbft::crypto::SecretKey,
        config: DkgModuleConfig,
        rendezvous_local_addr: SocketAddr,
        rendezvous_server_addr: SocketAddr,
        quic_port: u16,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<Event>,
    ) -> Result<DkgModule> {
        let engine = DkgEngine::new(
            node_idx,
            node_type,
            secret_key,
            ThresholdConfig {
                upper_bound: config.quorum_size as u16,
                threshold: config.quorum_threshold as u16,
            },
        );
        let socket_result = Socket::bind_with_config(
            rendezvous_local_addr,
            Config {
                blocking_mode: false,
                idle_connection_timeout: Duration::from_secs(5),
                heartbeat_interval: None,
                max_packet_size: (16 * 1024) as usize,
                max_fragments: 16 as u8,
                fragment_size: 1024,
                fragment_reassembly_buffer_size: 64,
                receive_buffer_max_size: 1452 as usize,
                rtt_smoothing_factor: 0.10,
                rtt_max_value: 250,
                socket_event_buffer_size: 1024,
                socket_polling_timeout: Some(Duration::from_millis(1000)),
                max_packets_in_flight: 512,
                max_unestablished_connections: 50,
            },
        );
        match socket_result {
            Ok(socket) => Ok(Self {
                dkg_engine: engine,
                quorum_type: config.quorum_type,
                rendezvous_local_addr,
                rendezvous_server_addr,
                quic_port,
                socket,
                status: ActorState::Stopped,
                label: String::from("State"),
                id: uuid::Uuid::new_v4().to_string(),
                broadcast_events_tx,
            }),
            Err(e) => Err(NodeError::Other(format!(
                "Error occurred while binding socket to port. Details :{0}",
                e.to_string()
            ))),
        }
    }

    #[cfg(test)]
    pub fn make_engine(
        dkg_engine: DkgEngine,
        events_tx: tokio::sync::mpsc::UnboundedSender<Event>,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<Event>,
    ) -> Self {
        let mut socket = Socket::bind_with_config(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            Config {
                blocking_mode: false,
                idle_connection_timeout: Duration::from_secs(5),
                heartbeat_interval: None,
                max_packet_size: (16 * 1024) as usize,
                max_fragments: 16 as u8,
                fragment_size: 1024,
                fragment_reassembly_buffer_size: 64,
                receive_buffer_max_size: 1452 as usize,
                rtt_smoothing_factor: 0.10,
                rtt_max_value: 250,
                socket_event_buffer_size: 1024,
                socket_polling_timeout: Some(Duration::from_millis(1000)),
                max_packets_in_flight: 512,
                max_unestablished_connections: 50,
            },
        )
            .unwrap();
        Self {
            dkg_engine,
            quorum_type: Some(QuorumType::Farmer),
            rendezvous_local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            rendezvous_server_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            quic_port: 9090,
            socket,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
            broadcast_events_tx,
        }
    }

    fn name(&self) -> String {
        String::from("DKG module")
    }

    pub fn process_rendezvous_response(&self) {
        let receiver = self.socket.get_event_receiver();
        let sender = self.socket.get_packet_sender();
        loop {
            if let Ok(event) = receiver.recv() {
                self.process_rendezvous_event(&event, &sender)
            }
        }
    }

    pub fn send_register_retrieve_peers_request(&self) {
        let sender = self.socket.get_packet_sender();

        let (tx1, rx1) = unbounded();
        let (tx2, rx2) = unbounded();

        // Spawning threads for retrieve peers request and register request
        DkgModule::spawn_interval_thread(
            Duration::from_secs(RETRIEVE_PEERS_REQUEST),
            tx1
        );

        DkgModule::spawn_interval_thread(
            Duration::from_secs(REGISTER_REQUEST),
            tx2
        );

        loop {
            select! {
                recv(rx1) -> _ => {
                    self.send_retrieve_peers_request(
                        &sender
                    );
                },
                recv(rx2) -> _ => {
                    self.send_register_request(
                        &sender,
                    );
                },
            }
        }
    }

    fn process_rendezvous_event(
        &self,
        event: &SocketEvent,
        sender: &Sender<Packet>
    ) {
        match event {
            SocketEvent::Packet(packet) => {
                self.process_packet(packet, sender)
            },
            SocketEvent::Timeout(_) => {},
            _ => {}
        }

    }

    fn process_packet(
        &self,
        packet: &Packet,
        sender: &Sender<Packet>
    ) {
        if packet.addr() == self.rendezvous_server_addr {
            if let Ok(payload_response) =
            bincode::deserialize::<Data>(packet.payload()) {
                self.process_payload_response(
                    &payload_response, sender, packet
                );
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
            Data::Request(req) => {
                self.process_request(req, sender, packet)
            },
            Data::Response(resp) => {
                self.process_response(resp)
            }
        }
    }

    fn process_request(
        &self,
        request: &RendezvousRequest,
        sender: &Sender<Packet>,
        packet: &Packet
    ) {
        match request {
            RendezvousRequest::Ping => {
                let response = &Data::Response(RendezvousResponse::Pong);
                if let Ok(data) = bincode::serialize(&response) {
                    let _ = sender.send(Packet::reliable_unordered(
                        packet.addr(),
                        data,
                    ));
                }
            },
            _ => {}
        }
    }

    fn process_response(
        &self,
        response: &RendezvousResponse,
    ) {
        match response {

            RendezvousResponse::Peers(peers) => {
                let _ = self.broadcast_events_tx
                    .send(Event::SyncPeers(peers.clone()));
            },
            RendezvousResponse::NamespaceRegistered => {
                info!("Namespace Registered");
            },
            RendezvousResponse::PeerRegistered => {
                info!("Peer Registered");
            },
            _ => {},
        }
    }

    fn send_retrieve_peers_request(
        &self,
        sender: &Sender<Packet>,
    ) {
        let quorum_key = if self.dkg_engine.node_type == NodeType::Farmer {
            self.dkg_engine.harvester_public_key
        } else {
            if let Some(key) = &self.dkg_engine.dkg_state.public_key_set {
                Some(key.public_key())
            } else {
                None
            }
        };

        if let Some(harvester_public_key) = quorum_key {
            if let Ok(data) = bincode::serialize(
                &Data::Request(
                    RendezvousRequest::Peers(
                        harvester_public_key.to_bytes().to_vec(),
                    )
                )
            ) {
                let _ = sender.send(
                    Packet::reliable_ordered(
                        self.rendezvous_server_addr,
                        data,
                        None,
                    )
                );
            }
        }
    }

    fn send_register_request(
        &self,
        sender: &Sender<Packet>,
    ) {
        match self.dkg_engine.dkg_state.public_key_set.clone() {
            Some(quorum_key) => {

                self.send_namespace_registration(
                    sender,
                    &quorum_key.public_key()
                );

                if let Some(secret_key_share) =
                self.dkg_engine.dkg_state.secret_key_share.clone()
                {
                    let (msg_bytes, signature) = self.generate_random_payload(
                        &secret_key_share
                    );
                    self.send_register_peer_payload(
                        sender,
                        &secret_key_share,
                        msg_bytes,
                        signature,
                        &quorum_key.public_key(),
                    );
                }
            }
            None => {
                error!("Cannot proceed with registration since current node is not part of any quorum");
            }
        }
    }

    fn send_namespace_registration(
        &self,
        sender: &Sender<Packet>,
        quorum_key: &PublicKey
    ) {

        if let Ok(data) = bincode::serialize(
            &Data::Request(
                RendezvousRequest::Namespace(
                    self.dkg_engine.node_type.to_string().as_bytes().to_vec(),
                    quorum_key.to_bytes().to_vec(),
                )
            )
        ) {
            let _ = sender.send(Packet::reliable_ordered(
                self.rendezvous_server_addr,
                data,
                None,
            ));
            thread::sleep(Duration::from_secs(5));
        }
    }

    fn send_register_peer_payload(
        &self,
        sender: &Sender<Packet>,
        secret_key_share: &SecretKeyShare,
        msg_bytes: Vec<u8>,
        signature: Vec<u8>,
        quorum_key: &PublicKey,
    ) {
        let payload_result = bincode::serialize(&Data::Request(
            RendezvousRequest::RegisterPeer(
                quorum_key.to_bytes().to_vec(),
                self.dkg_engine.node_type.to_string().as_bytes().to_vec(),
                secret_key_share.public_key_share().to_bytes().to_vec(),
                signature,
                msg_bytes,
                SyncPeerData {
                    address: self.rendezvous_local_addr,
                    raptor_udp_port: self.rendezvous_local_addr.port(),
                    quic_port: self.quic_port,
                    node_type: self.dkg_engine.node_type,
                },
            ),
        ));
        if let Ok(payload) = payload_result {
            let _ = sender.send(Packet::reliable_ordered(
                self.rendezvous_server_addr,
                payload,
                None,
            ));
        }
    }

    fn generate_random_payload(
        &self, secret_key_share: &SecretKeyShare
    ) -> (Vec<u8>, Vec<u8>) {

        let message: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();

        let msg_bytes = if let Ok(m) = hex::decode(message.clone()) {
            m
        } else {
            vec![]
        };

        let signature = secret_key_share.sign(
            message.clone()
        ).to_bytes().to_vec();

        (msg_bytes, signature)
    }

    fn spawn_interval_thread(interval: Duration, tx: Sender<()>) {
        thread::spawn(move || loop {
            sleep(interval);
            let _ = tx.send(());
        });
    }
}





#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Data {
    Request(RendezvousRequest),
    Response(RendezvousResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendezvousRequest {
    Ping,
    Peers(Vec<u8>),
    Namespace(NodeTypeBytes, QuorumPublicKey),
    RegisterPeer(
        QuorumPublicKey,
        NodeTypeBytes,
        PKShareBytes,
        RawSignature,
        PayloadBytes,
        SyncPeerData,
    ),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendezvousResponse {
    Pong,
    RequestPeers(QuorumPublicKey),
    Peers(Vec<SyncPeerData>),
    PeerRegistered,
    NamespaceRegistered,
}


#[async_trait]
impl Handler<Event> for DkgModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::DkgInitiate => {
                let threshold_config = self.dkg_engine.threshold_config.clone();
                if self.quorum_type.clone().is_some() {
                    match self
                        .dkg_engine
                        .generate_sync_keygen_instance(threshold_config.threshold as usize)
                    {
                        Ok(part_commitment) => {
                            if let DkgResult::PartMessageGenerated(node_idx, part) = part_commitment
                            {
                                if let Ok(part_committment_bytes) = bincode::serialize(&part) {
                                    let _ = self
                                        .broadcast_events_tx
                                        .send(Event::PartMessage(node_idx, part_committment_bytes));
                                }
                            }
                        },
                        Err(e) => {
                            error!("Error occured while generating synchronized keygen instance for node {:?}", self.dkg_engine.node_idx);
                        },
                    }
                } else {
                    error!(
                        "Cannot participate into DKG ,since current node {:?} dint win any Quorum Election",
                        self.dkg_engine.node_idx
                    );
                }
                return Ok(ActorState::Running);
            },
            Event::PartMessage(node_idx, part_committment_bytes) => {
                let part: bincode::Result<hbbft::sync_key_gen::Part> =
                    bincode::deserialize(&part_committment_bytes);
                if let Ok(part_committment) = part {
                    self.dkg_engine
                        .dkg_state
                        .part_message_store
                        .entry(node_idx)
                        .or_insert_with(|| part_committment);
                };
            },
            Event::AckPartCommitment(sender_id) => {
                if self
                    .dkg_engine
                    .dkg_state
                    .part_message_store
                    .contains_key(&sender_id)
                {
                    let dkg_result = self.dkg_engine.ack_partial_commitment(sender_id);
                    match dkg_result {
                        Ok(status) => match status {
                            DkgResult::PartMessageAcknowledged => {
                                if let Some(ack) = self
                                    .dkg_engine
                                    .dkg_state
                                    .ack_message_store
                                    .get(&(sender_id, self.dkg_engine.node_idx))
                                {
                                    if let Ok(ack_bytes) = bincode::serialize(&ack) {
                                        let _ = self.broadcast_events_tx.send(Event::SendAck(
                                            self.dkg_engine.node_idx,
                                            sender_id,
                                            ack_bytes,
                                        ));
                                    };
                                }
                            },
                            _ => {
                                error!("Error occured while acknowledging partial commitment for node {:?}", sender_id,);
                            },
                        },
                        Err(err) => {
                            error!("Error occured while acknowledging partial commitment for node {:?}: Err {:?}", sender_id, err);
                        },
                    }
                } else {
                    error!("Part Committment for Node idx {:?} missing ", sender_id);
                }
            },
            Event::HandleAllAcks => {
                let result = self.dkg_engine.handle_ack_messages();
                match result {
                    Ok(status) => {
                        info!("DKG Handle All Acks status {:?}", status);
                    },
                    Err(e) => {
                        error!("Error occured while handling all the acks {:?}", e);
                    },
                }
            },
            Event::GenerateKeySet => {
                let result = self.dkg_engine.generate_key_sets();
                match result {
                    Ok(status) => {
                        info!("DKG Completion status {:?}", status);
                    },
                    Err(e) => {
                        error!("Error occured while generating Quorum Public Key {:?}", e);
                    },
                }
            },
            Event::HarvesterPublicKey(key_bytes) => {
                let result: bincode::Result<PublicKey> = bincode::deserialize(&key_bytes);
                if let Ok(harvester_public_key) = result {
                    self.dkg_engine.harvester_public_key = Some(harvester_public_key);
                }
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use dkg_engine::test_utils;
    use events::Event;
    use hbbft::crypto::SecretKey;
    use primitives::{NodeType, QuorumType::Farmer};
    use theater::{Actor, ActorImpl};

    use super::*;

    #[tokio::test]
    async fn dkg_runtime_module_starts_and_stops() {
        let (broadcast_events_tx, broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<Event>();
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let dkg_config = DkgModuleConfig {
            quorum_type: Some(Farmer),
            quorum_size: 4,
            quorum_threshold: 2,
        };
        let sec_key: SecretKey = SecretKey::random();
        let dkg_module = DkgModule::new(
            1,
            NodeType::MasterNode,
            sec_key,
            dkg_config,
            "127.0.0.1:3031".parse().unwrap(),
            "127.0.0.1:3030".parse().unwrap(),
            9092,
            broadcast_events_tx,
        )
            .unwrap();
        let mut dkg_module = ActorImpl::new(dkg_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(dkg_module.status(), ActorState::Stopped);
        let handle = tokio::spawn(async move {
            dkg_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(dkg_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn dkg_runtime_dkg_init() {
        let (broadcast_events_tx, mut broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<Event>();

        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let dkg_config = DkgModuleConfig {
            quorum_type: Some(Farmer),
            quorum_size: 4,
            quorum_threshold: 2,
        };
        let sec_key: SecretKey = SecretKey::random();
        let mut dkg_module = DkgModule::new(
            1,
            NodeType::MasterNode,
            sec_key.clone(),
            dkg_config,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            9091,
            broadcast_events_tx,
        )
            .unwrap();
        dkg_module
            .dkg_engine
            .add_peer_public_key(1, sec_key.public_key());
        dkg_module
            .dkg_engine
            .add_peer_public_key(2, SecretKey::random().public_key());
        dkg_module
            .dkg_engine
            .add_peer_public_key(3, SecretKey::random().public_key());
        dkg_module
            .dkg_engine
            .add_peer_public_key(4, SecretKey::random().public_key());
        let mut dkg_module = ActorImpl::new(dkg_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(dkg_module.status(), ActorState::Stopped);
        let handle = tokio::spawn(async move {
            dkg_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(dkg_module.status(), ActorState::Terminating);
        });
        ctrl_tx.send(Event::DkgInitiate).unwrap();
        ctrl_tx.send(Event::AckPartCommitment(1)).unwrap();
        ctrl_tx.send(Event::Stop.into()).unwrap();
        let part_message_event = broadcast_events_rx.recv().await.unwrap();
        match part_message_event {
            Event::PartMessage(_, part_committment_bytes) => {
                let part_committment: bincode::Result<hbbft::sync_key_gen::Part> =
                    bincode::deserialize(&part_committment_bytes);
                assert!(part_committment.is_ok());
            },
            _ => {},
        }

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn dkg_runtime_dkg_ack() {
        let (broadcast_events_tx, mut broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<Event>();

        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let dkg_config = DkgModuleConfig {
            quorum_type: Some(Farmer),
            quorum_size: 4,
            quorum_threshold: 2,
        };
        let sec_key: SecretKey = SecretKey::random();
        let mut dkg_module = DkgModule::new(
            1,
            NodeType::MasterNode,
            sec_key.clone(),
            dkg_config,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            9092,
            broadcast_events_tx.clone(),
        )
            .unwrap();

        dkg_module
            .dkg_engine
            .add_peer_public_key(1, sec_key.public_key());

        dkg_module
            .dkg_engine
            .add_peer_public_key(2, SecretKey::random().public_key());

        dkg_module
            .dkg_engine
            .add_peer_public_key(3, SecretKey::random().public_key());

        dkg_module
            .dkg_engine
            .add_peer_public_key(4, SecretKey::random().public_key());

        let node_idx = dkg_module.dkg_engine.node_idx;
        let mut dkg_module = ActorImpl::new(dkg_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(20);

        assert_eq!(dkg_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            dkg_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(dkg_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::DkgInitiate).unwrap();
        let msg = broadcast_events_rx.recv().await.unwrap();
        if let Event::PartMessage(sender_id, part) = msg {
            assert_eq!(sender_id, 1);
            assert!(part.len() > 0);
        }
        ctrl_tx.send(Event::AckPartCommitment(1)).unwrap();
        let msg1 = broadcast_events_rx.recv().await.unwrap();
        if let Event::SendAck(curr_id, sender_id, ack) = msg1 {
            assert_eq!(curr_id, 1);
            assert_eq!(sender_id, 1);
            assert!(ack.len() > 0);
        }

        ctrl_tx.send(Event::Stop).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn dkg_runtime_handle_all_acks_generate_keyset() {
        let mut dkg_engines = test_utils::generate_dkg_engine_with_states().await;
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let (broadcast_events_tx, broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<Event>();
        let dkg_module =
            DkgModule::make_engine(dkg_engines.pop().unwrap(), events_tx, broadcast_events_tx);

        let mut dkg_module = ActorImpl::new(dkg_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(20);

        assert_eq!(dkg_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            dkg_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(dkg_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::HandleAllAcks).unwrap();
        ctrl_tx.send(Event::GenerateKeySet).unwrap();
        ctrl_tx.send(Event::Stop).unwrap();
        handle.await.unwrap();
    }
}