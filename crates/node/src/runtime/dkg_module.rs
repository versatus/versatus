use std::{hash::Hash, net::SocketAddr, path::PathBuf};

use async_trait::async_trait;
use dkg_engine::{
    dkg::DkgGenerator,
    types::{config::ThresholdConfig, DkgEngine, DkgError, DkgResult},
};
use hbbft::sync_key_gen::Part;
use kademlia_dht::{Key, Node, NodeData};
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::{NodeIdx, NodeType, QuorumType};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler};
use tracing::error;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};

use crate::{result::Result, NodeError, RuntimeModule};

pub struct DkgModuleConfig {
    pub quorum_type: Option<QuorumType>,
    pub quorum_size: usize,
    pub quorum_threshold: usize,
}

pub struct DkgModule {
    pub dkg_engine: DkgEngine,
    pub quorum_type: Option<QuorumType>,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

impl DkgModule {
    pub fn new(
        node_idx: NodeIdx,
        node_type: NodeType,
        secret_key: hbbft::crypto::SecretKey,
        config: DkgModuleConfig,
        events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    ) -> Self {
        let engine = DkgEngine::new(
            node_idx,
            node_type,
            secret_key,
            ThresholdConfig {
                upper_bound: config.quorum_size as u16,
                threshold: config.quorum_threshold as u16,
            },
        );
        Self {
            dkg_engine: engine,
            quorum_type: config.quorum_type,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            broadcast_events_tx,
        }
    }

    #[cfg(test)]
    pub fn make_engine(
        dkg_engine: DkgEngine,
        events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    ) -> Self {
        Self {
            dkg_engine,
            quorum_type: Some(QuorumType::Farmer),
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            broadcast_events_tx,
        }
    }

    fn name(&self) -> String {
        String::from("DKG module")
    }
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
                    let part_commitment = self
                        .dkg_engine
                        .generate_sync_keygen_instance(threshold_config.threshold as usize)
                        .unwrap();
                    if let DkgResult::PartMessageGenerated(node_idx, part) = part_commitment {
                        if let Ok(part_committment_bytes) = bincode::serialize(&part) {
                            let _ = self.broadcast_events_tx.send((
                                Topic::Network,
                                Event::PartMessage(node_idx, part_committment_bytes),
                            ));
                        }
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
                let threshold_config = self.dkg_engine.threshold_config.clone();
                if self.quorum_type.clone().is_some() {
                    let part_commitment = self
                        .dkg_engine
                        .generate_sync_keygen_instance(threshold_config.threshold as usize)
                        .unwrap();
                    if let DkgResult::PartMessageGenerated(node_idx, part) = part_commitment {
                        if let Ok(part_committment_bytes) = bincode::serialize(&part) {
                            let _ = self.broadcast_events_tx.send((
                                Topic::Network,
                                Event::PartMessage(node_idx, part_committment_bytes),
                            ));
                        }
                    }
                } else {
                    error!(
                                "Cannot participate into DKG ,since current node {:?} dint win any Quorum Election",
                                self.dkg_engine.node_idx
                            );
                }
                return Ok(ActorState::Running);
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
                                        let _ = self.broadcast_events_tx.send((
                                            Topic::Network,
                                            Event::SendAck(
                                                self.dkg_engine.node_idx,
                                                sender_id,
                                                ack_bytes,
                                            ),
                                        ));
                                    }
                                }
                            },
                            _ => {
                                error!("Error occured while acknowledging partial commitment for node {:?}", sender_id,);
                            },
                        },
                        Err(err) => {
                            error!("Error occured while acknowledging partial commitment for node {}: Err {}", sender_id, err);
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
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::{Borrow, BorrowMut},
        env,
        net::{IpAddr, Ipv4Addr},
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
        thread,
        time::Duration,
    };

    use dkg_engine::test_utils;
    use hbbft::crypto::SecretKey;
    use primitives::{NodeType, QuorumType::Farmer};
    use theater::ActorImpl;
    use tokio::{spawn, sync::mpsc::UnboundedReceiver};
    use vrrb_core::event_router::{DirectedEvent, Event, PeerData};

    use super::*;

    #[tokio::test]
    async fn dkg_runtime_module_starts_and_stops() {
        let (broadcast_events_tx, broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
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
            events_tx,
            broadcast_events_tx,
        );
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
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
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
            events_tx,
            broadcast_events_tx,
        );
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
        let part_message_event = broadcast_events_rx.recv().await.unwrap().1;
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
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
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
            events_tx,
            broadcast_events_tx.clone(),
        );

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
        let msg = broadcast_events_rx.recv().await.unwrap().1;
        if let Event::PartMessage(sender_id, part) = msg {
            assert_eq!(sender_id, 1);
            assert!(part.len() > 0);
        }
        ctrl_tx.send(Event::AckPartCommitment(1)).unwrap();
        let msg1 = broadcast_events_rx.recv().await.unwrap().1;
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
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (broadcast_events_tx, mut broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
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
