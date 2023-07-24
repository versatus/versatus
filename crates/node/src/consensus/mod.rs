mod consensus_component;
mod consensus_handler;
mod consensus_module;
mod quorum_component;
mod quorum_handler;

pub use consensus_component::*;
pub use consensus_handler::*;
pub use consensus_module::*;
pub use quorum_component::*;
pub use quorum_handler::*;

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        net::{IpAddr, Ipv4Addr},
        sync::{Arc, RwLock},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use bulldag::graph::BullDag;
    use dkg_engine::{test_utils, types::config::ThresholdConfig};
    use events::{Event, EventMessage, JobResult, DEFAULT_BUFFER};
    use hbbft::crypto::SecretKey;
    use lazy_static::lazy_static;
    use primitives::{Address, NodeType, QuorumType::Farmer};
    use secp256k1::Message;
    use signer::signer::SignatureProvider;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
    use theater::{Actor, ActorImpl, ActorState};
    use validator::validator_core_manager::ValidatorCoreManager;
    use vrrb_core::{
        account::Account,
        bloom::Bloom,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    // #[cfg(test)]
    // pub fn make_engine(
    //     dkg_engine: DkgEngine,
    //     _events_tx: EventPublisher,
    //     broadcast_events_tx: EventPublisher,
    // ) -> Self {
    //     use std::net::{IpAddr, Ipv4Addr};
    //
    //     let socket = Socket::bind_with_config(
    //         SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //         Config {
    //             blocking_mode: false,
    //             idle_connection_timeout: Duration::from_secs(5),
    //             heartbeat_interval: None,
    //             max_packet_size: (16 * 1024) as usize,
    //             max_fragments: 16_u8,
    //             fragment_size: 1024,
    //             fragment_reassembly_buffer_size: 64,
    //             receive_buffer_max_size: 1452_usize,
    //             rtt_smoothing_factor: 0.10,
    //             rtt_max_value: 250,
    //             socket_event_buffer_size: 1024,
    //             socket_polling_timeout: Some(Duration::from_millis(1000)),
    //             max_packets_in_flight: 512,
    //             max_unestablished_connections: 50,
    //         },
    //     )
    //     .unwrap();
    //
    //     Self {
    //         dkg_engine,
    //         quorum_type: Some(QuorumType::Farmer),
    //         rendezvous_local_addr:
    // SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    // rendezvous_server_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,
    // 0, 0, 1)), 0),         quic_port: 9090,
    //         socket,
    //         status: ActorState::Stopped,
    //         id: uuid::Uuid::new_v4().to_string(),
    //         broadcast_events_tx,
    //     }
    // }

    // use crate::componentx::{
    //     farmer_module::FarmerModule,
    //     scheduler::{Job, JobSchedulerController},
    // };
    //
    //     use super::*;
    //
    //     #[tokio::test]
    //     async fn dkg_runtime_module_starts_and_stops() {
    //         let (broadcast_events_tx, _broadcast_events_rx) =
    //             tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //         let (_events_tx, _) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();         let
    // dkg_config = DkgModuleConfig {             quorum_type: Some(Farmer),
    //             quorum_size: 4,
    //             quorum_threshold: 2,
    //         };
    //         let sec_key: SecretKey = SecretKey::random();
    //         let dkg_module = DkgModule::new(
    //             1,
    //             NodeType::MasterNode,
    //             sec_key,
    //             dkg_config,
    //             "127.0.0.1:3031".parse().unwrap(),
    //             "127.0.0.1:3030".parse().unwrap(),
    //             9092,
    //             broadcast_events_tx,
    //         )
    //         .unwrap();
    //
    //         let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //         let (ctrl_tx, mut ctrl_rx) =
    //
    // tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //         assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //         let handle = tokio::spawn(async move {
    //             dkg_module.start(&mut ctrl_rx).await.unwrap();
    //             assert_eq!(dkg_module.status(), ActorState::Terminating);
    //         });
    //
    //         ctrl_tx.send(Event::Stop.into()).unwrap();
    //         handle.await.unwrap();
    //     }
    //
    //     #[tokio::test]
    //     async fn dkg_runtime_dkg_init() {
    //         let (broadcast_events_tx, mut broadcast_events_rx) =
    //             tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //         let (_events_tx, _) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();         let
    // dkg_config = DkgModuleConfig {             quorum_type: Some(Farmer),
    //             quorum_size: 4,
    //             quorum_threshold: 2,
    //         };
    //         let sec_key: SecretKey = SecretKey::random();
    //         let mut dkg_module = DkgModule::new(
    //             1,
    //             NodeType::MasterNode,
    //             sec_key.clone(),
    //             dkg_config,
    //             SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //             SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //             9091,
    //             broadcast_events_tx,
    //         )
    //         .unwrap();
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(1, sec_key.public_key());
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(2, SecretKey::random().public_key());
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(3, SecretKey::random().public_key());
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(4, SecretKey::random().public_key());
    //         let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //         let (ctrl_tx, mut ctrl_rx) =
    //
    // tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //         assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //         let handle = tokio::spawn(async move {
    //             dkg_module.start(&mut ctrl_rx).await.unwrap();
    //             assert_eq!(dkg_module.status(), ActorState::Terminating);
    //         });
    //
    //         ctrl_tx.send(Event::DkgInitiate.into()).unwrap();
    //         ctrl_tx.send(Event::AckPartCommitment(1).into()).unwrap();
    //         ctrl_tx.send(Event::Stop.into()).unwrap();
    //
    //         let part_message_event =
    // broadcast_events_rx.recv().await.unwrap();         match
    // part_message_event.into() {             Event::PartMessage(_,
    // part_committment_bytes) => {                 let part_committment:
    // bincode::Result<hbbft::sync_key_gen::Part> =
    // bincode::deserialize(&part_committment_bytes);
    // assert!(part_committment.is_ok());             },
    //             _ => {},
    //         }
    //
    //         handle.await.unwrap();
    //     }
    //
    //     #[tokio::test]
    //     async fn dkg_runtime_dkg_ack() {
    //         let (broadcast_events_tx, mut broadcast_events_rx) =
    //             tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //         let (_events_tx, _) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();         let
    // dkg_config = DkgModuleConfig {             quorum_type: Some(Farmer),
    //             quorum_size: 4,
    //             quorum_threshold: 2,
    //         };
    //         let sec_key: SecretKey = SecretKey::random();
    //         let mut dkg_module = DkgModule::new(
    //             1,
    //             NodeType::MasterNode,
    //             sec_key.clone(),
    //             dkg_config,
    //             SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //             SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    //             9092,
    //             broadcast_events_tx.clone(),
    //         )
    //         .unwrap();
    //
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(1, sec_key.public_key());
    //
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(2, SecretKey::random().public_key());
    //
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(3, SecretKey::random().public_key());
    //
    //         dkg_module
    //             .dkg_engine
    //             .add_peer_public_key(4, SecretKey::random().public_key());
    //
    //         let _node_idx = dkg_module.dkg_engine.node_idx;
    //         let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //         let (ctrl_tx, mut ctrl_rx) =
    // tokio::sync::broadcast::channel::<EventMessage>(20);
    //
    //         assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //         let handle = tokio::spawn(async move {
    //             dkg_module.start(&mut ctrl_rx).await.unwrap();
    //             assert_eq!(dkg_module.status(), ActorState::Terminating);
    //         });
    //
    //         ctrl_tx.send(Event::DkgInitiate.into()).unwrap();
    //
    //         let msg = broadcast_events_rx.recv().await.unwrap();
    //         if let Event::PartMessage(sender_id, part) = msg.into() {
    //             assert_eq!(sender_id, 1);
    //             assert!(!part.is_empty());
    //         }
    //         ctrl_tx.send(Event::AckPartCommitment(1).into()).unwrap();
    //
    //         let msg1 = broadcast_events_rx.recv().await.unwrap();
    //
    //         if let Event::SendAck(curr_id, sender_id, ack) = msg1.into() {
    //             assert_eq!(curr_id, 1);
    //             assert_eq!(sender_id, 1);
    //             assert!(!ack.is_empty());
    //         }
    //
    //         ctrl_tx.send(Event::Stop.into()).unwrap();
    //
    //         handle.await.unwrap();
    //     }
    //
    //     #[tokio::test]
    //     async fn dkg_runtime_handle_all_acks_generate_keyset() {
    //         let mut dkg_engines =
    // test_utils::generate_dkg_engine_with_states().await;         let
    // (events_tx, _) =
    // tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //         let (broadcast_events_tx, _broadcast_events_rx) =
    //             tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //         let dkg_module =
    //             DkgModule::make_engine(dkg_engines.pop().unwrap(), events_tx,
    // broadcast_events_tx);
    //
    //         let mut dkg_module = ActorImpl::new(dkg_module);
    //
    //         let (ctrl_tx, mut ctrl_rx) =
    // tokio::sync::broadcast::channel::<EventMessage>(20);
    //
    //         assert_eq!(dkg_module.status(), ActorState::Stopped);
    //
    //         let handle = tokio::spawn(async move {
    //             dkg_module.start(&mut ctrl_rx).await.unwrap();
    //             assert_eq!(dkg_module.status(), ActorState::Terminating);
    //         });
    //
    //         ctrl_tx.send(Event::HandleAllAcks.into()).unwrap();
    //         ctrl_tx.send(Event::GenerateKeySet.into()).unwrap();
    //         ctrl_tx.send(Event::Stop.into()).unwrap();
    //
    //         handle.await.unwrap();
    //     }
    //
    //
    // #[tokio::test]
    // async fn farmer_module_starts_and_stops() {
    //     let (broadcast_events_tx, _) =
    // tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //     let (_, _clear_filter_rx) =
    // tokio::sync::mpsc::unbounded_channel::<Event>();
    //     let (sync_jobs_sender, _sync_jobs_receiver) =
    // crossbeam_channel::unbounded::<Job>();     let (async_jobs_sender,
    // _async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
    //
    //     let (_sync_jobs_status_sender, _sync_jobs_status_receiver) =
    //         crossbeam_channel::unbounded::<JobResult>();
    //     let (_async_jobs_status_sender, _async_jobs_status_receiver) =
    //         crossbeam_channel::unbounded::<JobResult>();
    //     let farmer_module = FarmerModule::new(
    //         None,
    //         vec![],
    //         vec![],
    //         0,
    //         broadcast_events_tx,
    //         2,
    //         sync_jobs_sender,
    //         async_jobs_sender,
    //     );
    //     let mut farmer_swarm_module = ActorImpl::new(farmer_module);
    //
    //     let (ctrl_tx, mut ctrl_rx) =
    //         tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     assert_eq!(farmer_swarm_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         farmer_swarm_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(farmer_swarm_module.status(),
    // ActorState::Terminating);     });
    //
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //     handle.await.unwrap();
    // }
    // lazy_static! {
    //     static ref STATE_SNAPSHOT: HashMap<Address, Account> =
    // HashMap::new(); }
    //
    // #[tokio::test]
    // async fn farmer_farm_cast_vote() {
    //     let (events_tx, _) =
    // tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (broadcast_events_tx, _broadcast_events_rx) =
    //         tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (_, _clear_filter_rx) =
    // tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (sync_jobs_sender, sync_jobs_receiver) =
    // crossbeam_channel::unbounded::<Job>();     let (async_jobs_sender,
    // async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
    //
    //     let mut db_config = VrrbDbConfig::default();
    //     let temp_dir_path = std::env::temp_dir();
    //     let db_path =
    // temp_dir_path.join(vrrb_core::helpers::generate_random_string());
    //     db_config.with_path(db_path);
    //
    //     let db = VrrbDb::new(db_config);
    //     let vrrbdb_read_handle = db.read_handle();
    //
    //     let mut job_scheduler = JobSchedulerController::new(
    //         vec![0],
    //         events_tx,
    //         sync_jobs_receiver,
    //         async_jobs_receiver,
    //         ValidatorCoreManager::new(8).unwrap(),
    //         vrrbdb_read_handle,
    //     );
    //     thread::spawn(move || {
    //         job_scheduler.execute_sync_jobs();
    //     });
    //     let mut dkg_engines =
    // test_utils::generate_dkg_engine_with_states().await;
    //     let dkg_engine = dkg_engines.pop().unwrap();
    //     let group_public_key = dkg_engine
    //         .dkg_state
    //         .public_key_set
    //         .clone()
    //         .unwrap()
    //         .public_key()
    //         .to_bytes()
    //         .to_vec();
    //     let sig_provider = SignatureProvider {
    //         dkg_state:
    // std::sync::Arc::new(std::sync::RwLock::new(dkg_engine.dkg_state)),
    //         quorum_config: ThresholdConfig {
    //             threshold: 2,
    //             upper_bound: 4,
    //         },
    //     };
    //     let mut farmer = FarmerModule::new(
    //         Some(sig_provider),
    //         group_public_key,
    //         dkg_engine.secret_key.public_key().to_bytes().to_vec(),
    //         1,
    //         broadcast_events_tx,
    //         2,
    //         sync_jobs_sender,
    //         async_jobs_sender,
    //     );
    //     let keypair = KeyPair::random();
    //     let recv_kp = KeyPair::random();
    //     let mut txns = HashSet::<Txn>::new();
    //
    //     let _now = SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .unwrap()
    //         .as_nanos();
    //
    //     // let txn_id = String::from("1");
    //     let sender_address = Address::new(*keypair.get_miner_public_key());
    //     let receiver_address = Address::new(*recv_kp.get_miner_public_key());
    //     let txn_amount: u128 = 1010101;
    //
    //     for n in 1..101 {
    //         let sig =
    // keypair.miner_kp.0.sign_ecdsa(Message::from_hashed_data::<
    //             secp256k1::hashes::sha256::Hash,
    //         >(
    //             b"
    // vrrb",
    //         ));
    //
    //         let txn = Txn::new(NewTxnArgs {
    //             timestamp: 0,
    //             sender_address: sender_address.clone(),
    //             sender_public_key: *keypair.get_miner_public_key(),
    //             receiver_address: receiver_address.clone(),
    //             token: None,
    //             amount: txn_amount + n,
    //             validators: Some(HashMap::<String, bool>::new()),
    //             nonce: 0,
    //             signature: sig,
    //         });
    //         txns.insert(txn);
    //     }
    //
    //     let _ = farmer.tx_mempool.extend(txns);
    //
    //     let mut farmer_swarm_module = ActorImpl::new(farmer);
    //     let (ctrl_tx, mut ctrl_rx) =
    // tokio::sync::broadcast::channel::<EventMessage>(10000);
    //     assert_eq!(farmer_swarm_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         farmer_swarm_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(farmer_swarm_module.status(),
    // ActorState::Terminating);     });
    //
    //     ctrl_tx.send(Event::Farm.into()).unwrap();
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //     handle.await.unwrap();
    // }

    // #[tokio::test]
    // async fn harvester_runtime_module_starts_and_stops() {
    //     let (broadcast_events_tx, _) =
    // tokio::sync::mpsc::channel(DEFAULT_BUFFER);
    //     let (sync_jobs_sender, _sync_jobs_receiver) =
    // crossbeam_channel::unbounded::<Job>();     let (async_jobs_sender,
    // _async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
    //     let (_, events_rx) =
    // tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     let (_sync_jobs_status_sender, _sync_jobs_status_receiver) =
    //         crossbeam_channel::unbounded::<JobResult>();
    //
    //     let (_async_jobs_status_sender, _async_jobs_status_receiver) =
    //         crossbeam_channel::unbounded::<JobResult>();
    //
    //     let mut db_config = VrrbDbConfig::default();
    //
    //     let temp_dir_path = std::env::temp_dir();
    //     let db_path =
    // temp_dir_path.join(vrrb_core::helpers::generate_random_string());
    //
    //     db_config.with_path(db_path);
    //
    //     let db = VrrbDb::new(db_config);
    //
    //     let vrrbdb_read_handle = db.read_handle();
    //
    //     let harvester_swarm_module = HarvesterModule::new(
    //         Bloom::new(10000),
    //         None,
    //         vec![],
    //         events_rx,
    //         broadcast_events_tx,
    //         2,
    //         Arc::new(RwLock::new(BullDag::new())),
    //         sync_jobs_sender,
    //         async_jobs_sender,
    //         vrrbdb_read_handle,
    //         Keypair::random(),
    //         1u16,
    //     );
    //     let mut harvester_swarm_module =
    // ActorImpl::new(harvester_swarm_module);
    //
    //     let (ctrl_tx, mut ctrl_rx) =
    //         tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
    //
    //     assert_eq!(harvester_swarm_module.status(), ActorState::Stopped);
    //
    //     let handle = tokio::spawn(async move {
    //         harvester_swarm_module.start(&mut ctrl_rx).await.unwrap();
    //         assert_eq!(harvester_swarm_module.status(),
    // ActorState::Terminating);     });
    //
    //     ctrl_tx.send(Event::Stop.into()).unwrap();
    //     handle.await.unwrap();
    // }
    // lazy_static! {
    //     static ref STATE_SNAPSHOT: HashMap<Address, Account> =
    // HashMap::new(); }
}
