use std::{collections::HashSet, net::SocketAddr, result::Result as StdResult, time::Duration};

use async_trait::async_trait;
use block::Block;
use bytes::Bytes;
use events::{DirectedEvent, Event};
use network::{
    message::{Message, MessageBody},
    network::BroadcastEngine,
};
use primitives::{NodeType, PeerId};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::{error, instrument};
use theater::{ActorLabel, ActorState, Handler};
use tokio::{
    sync::{
        broadcast::{
            error::{RecvError, TryRecvError},
            Receiver,
        },
        mpsc::{channel, Receiver as MpscReceiver, Sender},
    },
    task::JoinHandle,
    time::timeout,
};
use uuid::Uuid;

use crate::{NodeError, Result, RuntimeModuleState};

pub struct BroadcastModuleConfig {
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub udp_gossip_address_port: u16,
    pub raptorq_gossip_address_port: u16,
    pub node_id: PeerId,
}

// TODO: rename to GossipNetworkModule
#[derive(Debug)]
pub struct BroadcastModule {
    id: Uuid,
    status: ActorState,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    vrrbdb_read_handle: VrrbDbReadHandle,
    broadcast_engine: BroadcastEngine,
    status: ActorState,
}

const PACKET_TIMEOUT_DURATION: u64 = 10;

trait Timeout: Sized {
    fn timeout(self) -> tokio::time::Timeout<Self>;
}

impl<F: std::future::Future> Timeout for F {
    fn timeout(self) -> tokio::time::Timeout<Self> {
        tokio::time::timeout(Duration::from_secs(PACKET_TIMEOUT_DURATION), self)
    }
}

const PACKET_TIMEOUT_DURATION: u64 = 10;

trait Timeout: Sized {
    fn timeout(self) -> tokio::time::Timeout<Self>;
}

impl<F: std::future::Future> Timeout for F {
    fn timeout(self) -> tokio::time::Timeout<Self> {
        tokio::time::timeout(Duration::from_secs(PACKET_TIMEOUT_DURATION), self)
    }
}

impl BroadcastModule {
    pub async fn new(config: BroadcastModuleConfig) -> Result<Self> {
        let broadcast_engine = BroadcastEngine::new(config.udp_gossip_address_port, 32)
            .await
            .map_err(|err| {
                NodeError::Other(format!("unable to setup broadcast engine: {:?}", err))
            })?;

        Ok(Self {
            id: Uuid::new_v4(),
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            vrrbdb_read_handle: config.vrrbdb_read_handle,
            broadcast_engine,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.broadcast_engine.local_addr()
    }

    pub fn name(&self) -> String {
        "BroadcastModule".to_string()
    }

    pub async fn process_received_msg(&mut self) {
        loop {
            if let Some((_, mut incoming)) = self
                .broadcast_engine
                .get_incomming_connections()
                .next()
                .await
            {
                if let Ok(message_result) = incoming.next().timeout().await {
                    if let Ok(msg_option) = message_result {
                        if let Some(message) = msg_option {
                            let msg = Message::from_bytes(&message.2);
                            match msg.data {
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
                                MessageBody::DKGPartCommitment {
                                    part_commitment,
                                    sender_id,
                                } => {},
                                MessageBody::DKGPartAcknowledgement { .. } => {},
                                MessageBody::Vote { .. } => {},
                                MessageBody::Empty => {},
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The number of erasures that the raptorq encoder will use to encode the
/// block.
const RAPTOR_ERASURE_COUNT: u32 = 3000;

#[async_trait]
impl Handler<Event> for BroadcastModule {
    fn id(&self) -> theater::ActorId {
        self.id.to_string()
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

    #[instrument]
    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::PartMessage(sender_id, part_commitment) => {
                let status = self
                    .broadcast_engine
                    .quic_broadcast(Message::new(MessageBody::DKGPartCommitment {
                        sender_id,
                        part_commitment,
                    }))
                    .await;
                match status {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "Error occured while broadcasting ack commitment to peers :{:?}",
                            e
                        );
                    },
                }
            },
            Event::SendAck(curr_node_id, sender_id, ack) => {
                let status = self
                    .broadcast_engine
                    .quic_broadcast(Message::new(MessageBody::DKGPartAcknowledgement {
                        curr_node_id,
                        sender_id,
                        ack,
                    }))
                    .await;
                match status {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "Error occured while broadcasting Part commitment to peers :{:?}",
                            e
                        );
                    },
                }
            },
            Event::SyncPeers(peers) => {
                let mut quic_addresses = vec![];
                let mut raptor_peer_list = vec![];
                for peer in peers.iter() {
                    if let Ok(addr) = peer.address.parse::<SocketAddr>() {
                        quic_addresses.push(addr);
                        let mut raptor_addr = addr.clone();
                        raptor_addr.set_port(peer.raptor_udp_port);
                        raptor_peer_list.push(raptor_addr);
                    }
                }
                self.broadcast_engine.add_raptor_peers(raptor_peer_list);
                self.broadcast_engine.add_peer_connection(quic_addresses);
            },
            Event::Vote(vote, quorum_type, farmer_quorum_threshold) => {
                let status = self
                    .broadcast_engine
                    .quic_broadcast(Message::new(MessageBody::Vote {
                        vote,
                        quorum_type,
                        farmer_quorum_threshold,
                    }))
                    .await;
                match status {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "Error occured while broadcasting votes to harvesters :{:?}",
                            e
                        );
                    },
                }
            },
            /// Broadcasting the Convergence block to the peers.
            Event::BlockConfirmed(block) => {
                let status = self
                    .broadcast_engine
                    .unreliable_broadcast(
                        block,
                        RAPTOR_ERASURE_COUNT,
                        self.broadcast_engine.raptor_udp_port,
                    )
                    .await;
                match status {
                    Ok(_) => {},
                    Err(e) => {
                        error!("Error occured while broadcasting blocks to peers :{:?}", e);
                    },
                }
            },

            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::io::stdout;
    use events::Event;
    use primitives::NodeType;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
    use tokio::sync::mpsc::unbounded_channel;

    use super::{BroadcastModule, BroadcastModuleConfig};

    #[tokio::test]
    async fn test_broadcast_module() {
        let (internal_events_tx, mut internal_events_rx) = unbounded_channel();

        let node_id = uuid::Uuid::new_v4().to_string().into_bytes();

        let mut db_config = VrrbDbConfig::default();

        let temp_dir_path = std::env::temp_dir();
        let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

        db_config.with_path(db_path);

        let db = VrrbDb::new(db_config);

        let vrrbdb_read_handle = db.read_handle();

        let config = BroadcastModuleConfig {
            events_tx: internal_events_tx,
            vrrbdb_read_handle,
            node_type: NodeType::Full,
            udp_gossip_address_port: 0,
            raptorq_gossip_address_port: 0,
            node_id,
        };

        let (events_tx, mut events_rx) = tokio::sync::broadcast::channel::<Event>(10);

        let broadcast_module = BroadcastModule::new(config).await.unwrap();

        let mut broadcast_module_actor = ActorImpl::new(broadcast_module);

        let handle = tokio::spawn(async move {
            broadcast_module_actor.start(&mut events_rx).await.unwrap();
        });

        let bound_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        let address = bound_socket.local_addr().unwrap();

        let peer_data = SyncPeerData {
            address,
            raptor_udp_port: 9993,
            quic_port: 9994,
            node_type: NodeType::Full,
        };

        events_tx.send(Event::SyncPeers(vec![peer_data])).unwrap();
        events_tx.send(Event::Stop).unwrap();

        let evt = internal_events_rx.recv().await.unwrap();

        handle.await.unwrap();
    }
}
