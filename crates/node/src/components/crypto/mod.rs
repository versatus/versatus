use std::{net::SocketAddr, thread, thread::sleep, time::Duration};

use async_trait::async_trait;
use crossbeam_channel::{select, unbounded, Sender};
use dkg_engine::{
    dkg::DkgGenerator,
    types::{config::ThresholdConfig, DkgEngine, DkgResult},
};
use events::{Event, EventMessage, EventPublisher, SyncPeerData};
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
use theater::{ActorId, ActorLabel, ActorState, Handler, TheaterError};
use tracing::error;

use crate::{result::Result, NodeError};

// mod dkg_module;
// pub use dkg_module::*;
mod crypto_module;
pub use crypto_module::*;

#[cfg(test)]
mod tests {
    //
    // use std::net::{IpAddr, Ipv4Addr}
    //
    // use dkg_engine::test_utils;
    // use events::{Event, DEFAULT_BUFFER};
    // use hbbft::crypto::SecretKey;
    // use primitives::{NodeType, QuorumType::Farmer};
    // use theater::{Actor, ActorImpl};
    //
    // use super::*;
    //
    // #[tokio::test]
    // async fn dkg_runtime_module_starts_and_stops() {
    //     let (broadcast_events_tx, _broadcast_events_rx) =
    //         tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (_events_tx, _) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();     let dkg_config =
    // DkgModuleConfig {         quorum_type: Some(Farmer),
    //         quorum_size: 4,
    //         quorum_threshold: 2,
    //     };
    //
    //     let sec_key: SecretKey = SecretKey::random();
    //     let dkg_module = DkgModule::new(
    //         1,
    //         NodeType::MasterNode,
    //         sec_key,
    //         dkg_config,
    //         "127.0.0.1:3031".parse().unwrap(),
    //         "127.0.0.1:3030".parse().unwrap(),
    //         9092,
    //         broadcast_events_tx,
    //     )
    //     .unwrap();
    //
    //     let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //     let (ctrl_tx, mut ctrl_rx) =
    //         tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         dkg_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(dkg_module.status(), ActorState::Terminating);
    //     });
    //
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //     handle.await.unwrap();
    // }
    //
    // #[tokio::test]
    // async fn dkg_runtime_dkg_init() {
    //     let (broadcast_events_tx, mut broadcast_events_rx) =
    //         tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (_events_tx, _) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();     let dkg_config =
    // DkgModuleConfig {         quorum_type: Some(Farmer),
    //         quorum_size: 4,
    //         quorum_threshold: 2,
    //     };
    //     let sec_key: SecretKey = SecretKey::random();
    //     let mut dkg_module = DkgModule::new(
    //         1,
    //         NodeType::MasterNode,
    //         sec_key.clone(),
    //         dkg_config,
    //         SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //         SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //         9091,
    //         broadcast_events_tx,
    //     )
    //     .unwrap();
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(1, sec_key.public_key());
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(2, SecretKey::random().public_key());
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(3, SecretKey::random().public_key());
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(4, SecretKey::random().public_key());
    //     let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //     let (ctrl_tx, mut ctrl_rx) =
    //         tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         dkg_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(dkg_module.status(), ActorState::Terminating);
    //     });
    //
    //     ctrl_tx.send(Event::DkgInitiate.into()).unwrap();
    //     ctrl_tx.send(Event::AckPartCommitment(1).into()).unwrap();
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //
    //     let part_message_event = broadcast_events_rx.recv().await.unwrap();
    //     match part_message_event.into() {
    //         Event::PartMessage(_, part_committment_bytes) => {
    //             let part_committment:
    // bincode::Result<hbbft::sync_key_gen::Part> =
    // bincode::deserialize(&part_committment_bytes);
    // assert!(part_committment.is_ok());         },
    //         _ => {},
    //     }
    //
    //     handle.await.unwrap();
    // }
    //
    // #[tokio::test]
    // async fn dkg_runtime_dkg_ack() {
    //     let (broadcast_events_tx, mut broadcast_events_rx) =
    //         tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (_events_tx, _) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();     let dkg_config =
    // DkgModuleConfig {         quorum_type: Some(Farmer),
    //         quorum_size: 4,
    //         quorum_threshold: 2,
    //     };
    //     let sec_key: SecretKey = SecretKey::random();
    //     let mut dkg_module = DkgModule::new(
    //         1,
    //         NodeType::MasterNode,
    //         sec_key.clone(),
    //         dkg_config,
    //         SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //         SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //         9092,
    //         broadcast_events_tx.clone(),
    //     )
    //     .unwrap();
    //
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(1, sec_key.public_key());
    //
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(2, SecretKey::random().public_key());
    //
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(3, SecretKey::random().public_key());
    //
    //     dkg_module
    //         .dkg_engine
    //         .add_peer_public_key(4, SecretKey::random().public_key());
    //
    //     let _node_idx = dkg_module.dkg_engine.node_idx;
    //     let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //     let (ctrl_tx, mut ctrl_rx) =
    // tokio::sync::broadcast::channel::<EventMessage>(20);
    //
    //     assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         dkg_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(dkg_module.status(), ActorState::Terminating);
    //     });
    //
    //     ctrl_tx.send(Event::DkgInitiate.into()).unwrap();
    //
    //     let msg = broadcast_events_rx.recv().await.unwrap();
    //     if let Event::PartMessage(sender_id, part) = msg.into() {
    //         assert_eq!(sender_id, 1);
    //         assert!(!part.is_empty());
    //     }
    //     ctrl_tx.send(Event::AckPartCommitment(1).into()).unwrap();
    //
    //     let msg1 = broadcast_events_rx.recv().await.unwrap();
    //
    //     if let Event::SendAck(curr_id, sender_id, ack) = msg1.into() {
    //         assert_eq!(curr_id, 1);
    //         assert_eq!(sender_id, 1);
    //         assert!(!ack.is_empty());
    //     }
    //
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //
    //     handle.await.unwrap();
    // }
    //
    // #[tokio::test]
    // async fn dkg_runtime_handle_all_acks_generate_keyset() {
    //     let mut dkg_engines =
    // test_utils::generate_dkg_engine_with_states().await;
    //     let (events_tx, _) =
    // tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //     let (broadcast_events_tx, _broadcast_events_rx) =
    //         tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let dkg_module =
    //         DkgModule::make_engine(dkg_engines.pop().unwrap(), events_tx,
    // broadcast_events_tx);
    //
    //     let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //     let (ctrl_tx, mut ctrl_rx) =
    // tokio::sync::broadcast::channel::<EventMessage>(20);
    //
    //     assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         dkg_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(dkg_module.status(), ActorState::Terminating);
    //     });
    //
    //     ctrl_tx.send(Event::HandleAllAcks.into()).unwrap();
    //     ctrl_tx.send(Event::GenerateKeySet.into()).unwrap();
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //
    //     handle.await.unwrap();
    // }
}
