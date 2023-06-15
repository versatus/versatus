// let (state_read_handle, state_handle) = setup_state_store(
//     &config,
//     events_tx.clone(),
//     vrrbdb_events_rx,
//     dag.clone(),
//     mempool_read_handle_factory.clone(),
// )
// .await?;

// let mut gossip_handle = None;
// let (raptor_sender, raptor_receiver) = unbounded::<RaptorBroadCastedData>();
// if !config.disable_networking {
//     let (new_gossip_handle, _, gossip_addr) = setup_gossip_network(
//         &config,
//         events_tx.clone(),
//         network_events_rx,
//         controller_events_rx,
//         state_read_handle.clone(),
//         raptor_sender,
//     )
//     .await?;
//
//     gossip_handle = new_gossip_handle;
//     config.udp_gossip_address = gossip_addr;
// }
//
// let raptor_handle = thread::spawn({
//     let events_tx = events_tx.clone();
//     move || {
//         let events_tx = events_tx.clone();
//         loop {
//             let events_tx = events_tx.clone();
//             if let Ok(data) = raptor_receiver.recv() {
//                 match data {
//                     RaptorBroadCastedData::Block(block) => {
//                         tokio::spawn(async move {
//                             let _ =
// events_tx.send(Event::BlockReceived(block).into()).await;
// });                     },
//                 }
//             }
//         }
//     }
// });
//
// async fn setup_gossip_network(
//     config: &NodeConfig,
//     events_tx: EventPublisher,
//     mut network_events_rx: EventSubscriber,
//     controller_events_rx: EventSubscriber,
//     vrrbdb_read_handle: VrrbDbReadHandle,
//     raptor_sender: Sender<RaptorBroadCastedData>,
// ) -> Result<(
//     Option<JoinHandle<Result<()>>>,
//     Option<JoinHandle<(Result<()>, Result<()>)>>,
//     SocketAddr,
// )> {
//     let broadcast_module = BroadcastModule::new(BroadcastModuleConfig {
//         events_tx: events_tx.clone(),
//         vrrbdb_read_handle,
//         udp_gossip_address_port: config.udp_gossip_address.port(),
//         raptorq_gossip_address_port: config.raptorq_gossip_address.port(),
//         node_type: config.node_type,
//         node_id: config.id.as_bytes().to_vec(),
//     })
//     .await?;
//
//     let addr = broadcast_module.local_addr();
//
//     let broadcast_engine =
// BroadcastEngine::new(config.udp_gossip_address.port(), 32)         .await
//         .map_err(|err| NodeError::Other(format!("unable to setup broadcast
// engine: {:?}", err)))?;
//
//     let broadcast_resolved_addr = broadcast_engine.local_addr();
//
//     let mut bcast_controller = BroadcastEngineController::new(
//         BroadcastEngineControllerConfig::new(broadcast_engine,
// events_tx.clone()),     );
//
//     let broadcast_controller_handle = tokio::spawn(async move {
//         let broadcast_handle =
// bcast_controller.listen(controller_events_rx).await;         let
// raptor_handle = bcast_controller             .engine
//
// .process_received_packets(bcast_controller.engine.raptor_udp_port,
// raptor_sender)             .await;
//
//         let raptor_handle = raptor_handle.map_err(NodeError::Broadcast);
//         (broadcast_handle, raptor_handle)
//     });
//
//     let mut broadcast_module_actor = ActorImpl::new(broadcast_module);
//
//     let broadcast_handle = tokio::spawn(async move {
//         broadcast_module_actor
//             .start(&mut network_events_rx)
//             .await
//             .map_err(|err| NodeError::Other(err.to_string()))
//     });
//
//     info!("Broadcast engine listening on {}", broadcast_resolved_addr);
//
//     Ok((
//         Some(broadcast_handle),
//         Some(broadcast_controller_handle),
//         addr,
//     ))
// }

// async fn setup_state_store(
//     config: &NodeConfig,
//     events_tx: EventPublisher,
//     mut state_events_rx: EventSubscriber,
//     dag: Arc<RwLock<BullDag<Block, String>>>,
//     _mempool_read_handle_factory: MempoolReadHandleFactory,
// ) -> Result<(VrrbDbReadHandle, Option<JoinHandle<Result<()>>>)> {
//     let mut vrrbdb_config = VrrbDbConfig::default();
//
//     if config.db_path() != &vrrbdb_config.path {
//         vrrbdb_config.with_path(config.db_path().to_path_buf());
//     }
//
//     let db = storage::vrrbdb::VrrbDb::new(vrrbdb_config);
//
//     let vrrbdb_read_handle = db.read_handle();
//
//     let state_module = StateModule::new(state_module::StateModuleConfig {
// events_tx, db, dag });
//
//     let mut state_module_actor = ActorImpl::new(state_module);
//
//     let state_handle = tokio::spawn(async move {
//         state_module_actor
//             .start(&mut state_events_rx)
//             .await
//             .map_err(|err| NodeError::Other(err.to_string()))
//     });
//
//     info!("State store is operational");
//
//     Ok((vrrbdb_read_handle, Some(state_handle)))
// }
//
// async fn setup_grpc_api_server(
//     config: &NodeConfig,
//     events_tx: EventPublisher,
//     vrrbdb_read_handle: VrrbDbReadHandle,
//     mempool_read_handle_factory: MempoolReadHandleFactory,
//     // mut jsonrpc_events_rx: EventSubscriber,
// ) -> Result<(Option<JoinHandle<Result<()>>>, SocketAddr)> {
//     let grpc_server_config = GrpcServerConfig {
//         address: config.grpc_server_address,
//         node_type: config.node_type,
//         events_tx,
//         vrrbdb_read_handle,
//         mempool_read_handle_factory,
//     };
//
//     let address = grpc_server_config.address;
//
//     let handle = tokio::spawn(async move {
//         let resolved_grpc_server_addr = GrpcServer::run(&grpc_server_config)
//             .await
//             .map_err(|err| NodeError::Other(format!("unable to start gRPC
// server, {}", err)))             .expect("gRPC server to start");
//
//         Ok(())
//     });
//
//     info!("gRPC server started at {}", &address);
//
//     Ok((Some(handle), address))
// }
// use std::net::SocketAddr;
//
// use bytes::Bytes;
// use events::{Event, EventMessage, EventPublisher, EventSubscriber};
// use network::{
//     message::{Message, MessageBody},
//     network::{BroadcastEngine, ConnectionIncoming},
// };
// use telemetry::{error, info, warn};
//
// use crate::{NodeError, Result};
//
// /// The number of erasures that the raptorq encoder will use to encode the
// /// block.
// const RAPTOR_ERASURE_COUNT: u32 = 3000;
//
// #[derive(Debug)]
// pub struct BroadcastEngineController {
//     pub engine: BroadcastEngine,
//     events_tx: EventPublisher,
// }
//
// #[derive(Debug)]
// pub struct BroadcastEngineControllerConfig {
//     pub engine: BroadcastEngine,
//     pub events_tx: EventPublisher,
// }
//
// impl BroadcastEngineControllerConfig {
//     pub fn new(engine: BroadcastEngine, events_tx: EventPublisher) -> Self {
//         Self { engine, events_tx }
//     }
//
//     pub fn _local_addr(&self) -> SocketAddr {
//         self.engine.local_addr()
//     }
// }
//
// impl BroadcastEngineController {
//     pub fn new(config: BroadcastEngineControllerConfig) -> Self {
//         let engine = config.engine;
//         let events_tx = config.events_tx;
//
//         Self { engine, events_tx }
//     }
//
//     pub async fn listen(&mut self, mut events_rx: EventSubscriber) ->
// Result<()> {         loop {
//             tokio::select! {
//                 Some((_conn, conn_incoming)) =
// self.engine.get_incoming_connections().next() => {                 match
// self.map_network_conn_to_message(conn_incoming).await {                     
// Ok(message) => {                         
// self.handle_network_event(message).await?;                     },
//                      Err(err) => {
//                         error!("unable to map connection into message:
// {err}");                     }
//                   }
//                 },
//                 Ok(event) = events_rx.recv() => {
//                     if matches!(event.clone().into(), Event::Stop) {
//                         info!("Stopping broadcast controller");
//                         break
//                     }
//                     self.handle_internal_event(event.into()).await?;
//                 },
//             };
//         }
//
//         Ok(())
//     }
//
//     async fn handle_network_event(&self, message: Message) -> Result<()> {
//         match message.data {
//             MessageBody::InvalidBlock { .. } => {},
//             MessageBody::Disconnect { .. } => {},
//             MessageBody::StateComponents { .. } => {},
//             MessageBody::Genesis { .. } => {},
//             MessageBody::Child { .. } => {},
//             MessageBody::Parent { .. } => {},
//             MessageBody::Ledger { .. } => {},
//             MessageBody::NetworkState { .. } => {},
//             MessageBody::ClaimAbandoned { .. } => {},
//             MessageBody::ResetPeerConnection { .. } => {},
//             MessageBody::RemovePeer { .. } => {},
//             MessageBody::AddPeer { .. } => {},
//             MessageBody::DKGPartCommitment { .. } => {},
//             MessageBody::DKGPartAcknowledgement { .. } => {},
//             MessageBody::ForwardedTxn(txn_record) => {
//                 info!("Received Forwarded Txn :{:?}", txn_record.txn_id);
//                 let _ = self
//                     .events_tx
//                     .send(EventMessage::new(
//                         None,
//                         Event::NewTxnCreated(txn_record.txn),
//                     ))
//                     .await;
//             },
//             MessageBody::Vote { .. } => {},
//             MessageBody::Empty => {},
//         };
//
//         Ok(())
//     }
//
//     async fn handle_internal_event(&mut self, event: Event) -> Result<()> {
//         match event {
//             Event::Stop => Ok(()),
//             Event::PartMessage(sender_id, part_commitment) => {
//                 let status = self
//                     .engine
//                     
// .quic_broadcast(Message::new(MessageBody::DKGPartCommitment {                
// sender_id,                         part_commitment,
//                     }))
//                     .await?;
//
//                 info!("Broadcasted part commitment to peers: {status:?}");
//                 Ok(())
//             },
//             Event::SyncPeers(peers) => {
//                 if peers.is_empty() {
//                     warn!("No peers to sync with");
//
//                     self.events_tx.send(Event::EmptyPeerSync.into()).await?;
//
//                     // TODO: revisit this return
//                     return Ok(());
//                 }
//
//                 let mut quic_addresses = vec![];
//                 let mut raptor_peer_list = vec![];
//
//                 for peer in peers.iter() {
//                     let addr = peer.address;
//
//                     quic_addresses.push(addr);
//
//                     let mut raptor_addr = addr;
//                     raptor_addr.set_port(peer.raptor_udp_port);
//                     raptor_peer_list.push(raptor_addr);
//                 }
//
//                 self.engine.add_raptor_peers(raptor_peer_list);
//
//                 let peer_connection_result = self
//                     .engine
//                     .add_peer_connection(quic_addresses.clone())
//                     .await;
//
//                 if let Err(err) = peer_connection_result {
//                     error!("unable to add peer connection: {err}");
//
//                     self.events_tx
//                         .send(Event::PeerSyncFailed(quic_addresses).into())
//                         .await?;
//
//                     return Err(err.into());
//                 }
//
//                 if let Ok(status) = peer_connection_result {
//                     info!("{status:?}");
//                 }
//
//                 Ok(())
//             },
//             Event::Vote(vote, farmer_quorum_threshold) => {
//                 let status = self
//                     .engine
//                     .quic_broadcast(Message::new(MessageBody::Vote {
//                         vote,
//                         farmer_quorum_threshold,
//                     }))
//                     .await?;
//
//                 info!("{status:?}");
//
//                 Ok(())
//             },
//             // Broadcasting the Convergence block to the peers.
//             Event::BlockConfirmed(block) => {
//                 let status = self
//                     .engine
//                     .unreliable_broadcast(block, RAPTOR_ERASURE_COUNT,
// self.engine.raptor_udp_port)                     .await?;
//
//                 info!("{status:?}");
//
//                 Ok(())
//             },
//             _ => Ok(()),
//         }
//     }
//
//     /// Turns connection data into Message then returns it
//     async fn map_network_conn_to_message(
//         &self,
//         mut conn_incoming: ConnectionIncoming,
//     ) -> Result<Message> {
//         let res = conn_incoming.next().await.map_err(|err| {
//             NodeError::Other(format!("unable to listen for new connections:
// {err}"))         })?;
//
//         let (_, _, raw_message) = res.unwrap_or((Bytes::new(), Bytes::new(),
// Bytes::new()));         let message = Message::from(raw_message.to_vec());
//
//         Ok(message)
//     }
// }
//
// use std::{net::SocketAddr, time::Duration};
//
// use async_trait::async_trait;
// use events::{Event, EventMessage, EventPublisher};
// use network::{
//     message::{Message, MessageBody},
//     network::BroadcastEngine,
// };
// use primitives::{NodeType, PeerId};
// use storage::vrrbdb::VrrbDbReadHandle;
// use telemetry::{error, info, instrument};
// use theater::{ActorLabel, ActorState, Handler};
// use uuid::Uuid;
//
// use crate::{NodeError, Result};
//
// pub struct BroadcastModuleConfig {
//     pub events_tx: EventPublisher,
//     pub node_type: NodeType,
//     pub vrrbdb_read_handle: VrrbDbReadHandle,
//     pub udp_gossip_address_port: u16,
//     pub raptorq_gossip_address_port: u16,
//     pub node_id: PeerId,
// }
//
// // TODO: rename to GossipNetworkModule
// #[derive(Debug)]
// pub struct BroadcastModule {
//     id: Uuid,
//     status: ActorState,
//     events_tx: EventPublisher,
//     _vrrbdb_read_handle: VrrbDbReadHandle,
//     broadcast_engine: BroadcastEngine,
// }
//
// const PACKET_TIMEOUT_DURATION: u64 = 10;
//
// trait Timeout: Sized {
//     fn timeout(self) -> tokio::time::Timeout<Self>;
// }
//
// impl<F: std::future::Future> Timeout for F {
//     fn timeout(self) -> tokio::time::Timeout<Self> {
//         tokio::time::timeout(Duration::from_secs(PACKET_TIMEOUT_DURATION),
// self)     }
// }
//
// impl BroadcastModule {
//     pub async fn new(config: BroadcastModuleConfig) -> Result<Self> {
//         let broadcast_engine =
// BroadcastEngine::new(config.udp_gossip_address_port, 32)             .await
//             .map_err(|err| {
//                 NodeError::Other(format!("unable to setup broadcast engine:
// {err:?}"))             })?;
//
//         Ok(Self {
//             id: Uuid::new_v4(),
//             events_tx: config.events_tx,
//             status: ActorState::Stopped,
//             _vrrbdb_read_handle: config.vrrbdb_read_handle,
//             broadcast_engine,
//         })
//     }
//
//     pub fn local_addr(&self) -> SocketAddr {
//         self.broadcast_engine.local_addr()
//     }
//
//     pub fn name(&self) -> String {
//         "Broadcast".to_string()
//     }
//
//     pub async fn process_received_msg(&mut self) {
//         loop {
//             if let Some((_, mut incoming)) = self
//                 .broadcast_engine
//                 .get_incoming_connections()
//                 .next()
//                 .await
//             {
//                 if let Ok(Ok(Some(message))) =
// incoming.next().timeout().await {                     let msg =
// Message::from_bytes(&message.2);                     match msg.data {
//                         MessageBody::InvalidBlock { .. } => {},
//                         MessageBody::Disconnect { .. } => {},
//                         MessageBody::StateComponents { .. } => {},
//                         MessageBody::Genesis { .. } => {},
//                         MessageBody::Child { .. } => {},
//                         MessageBody::Parent { .. } => {},
//                         MessageBody::Ledger { .. } => {},
//                         MessageBody::NetworkState { .. } => {},
//                         MessageBody::ClaimAbandoned { .. } => {},
//                         MessageBody::ResetPeerConnection { .. } => {},
//                         MessageBody::RemovePeer { .. } => {},
//                         MessageBody::AddPeer { .. } => {},
//                         MessageBody::DKGPartCommitment {
//                             part_commitment: _,
//                             sender_id: _,
//                         } => {},
//                         MessageBody::DKGPartAcknowledgement { .. } => {},
//                         MessageBody::Vote { .. } => {},
//                         MessageBody::Empty => {},
//                         MessageBody::ForwardedTxn(txn) => {
//                             let _ = self
//                                 .events_tx
//                                 .send(EventMessage::new(None,
// Event::NewTxnCreated(txn.txn)))                                 .await;
//                         },
//                     }
//                 }
//             }
//         }
//     }
// }
//
// /// The number of erasures that the raptorq encoder will use to encode the
// /// block.
// const RAPTOR_ERASURE_COUNT: u32 = 3000;
//
// #[async_trait]
// impl Handler<EventMessage> for BroadcastModule {
//     fn id(&self) -> theater::ActorId {
//         self.id.to_string()
//     }
//
//     fn label(&self) -> ActorLabel {
//         self.name()
//     }
//
//     fn status(&self) -> ActorState {
//         self.status.clone()
//     }
//
//     fn set_status(&mut self, actor_status: ActorState) {
//         self.status = actor_status;
//     }
//
//     fn on_start(&self) {
//         info!("{}-{} starting", self.label(), self.id(),);
//     }
//
//     #[instrument]
//     async fn handle(&mut self, event: EventMessage) ->
// theater::Result<ActorState> {         match event.into() {
//             Event::Stop => {
//                 return Ok(ActorState::Stopped);
//             },
//             Event::PartMessage(sender_id, part_commitment) => {
//                 let status = self
//                     .broadcast_engine
//                     
// .quic_broadcast(Message::new(MessageBody::DKGPartCommitment {                
// sender_id,                         part_commitment,
//                     }))
//                     .await;
//                 match status {
//                     Ok(_) => {},
//                     Err(e) => {
//                         error!(
//                             "Error occured while broadcasting ack commitment
// to peers :{:?}",                             e
//                         );
//                     },
//                 }
//             },
//             Event::SendAck(curr_node_id, sender_id, ack) => {
//                 let status = self
//                     .broadcast_engine
//                     
// .quic_broadcast(Message::new(MessageBody::DKGPartAcknowledgement {
//                         curr_node_id,
//                         sender_id,
//                         ack,
//                     }))
//                     .await;
//                 match status {
//                     Ok(_) => {},
//                     Err(e) => {
//                         error!(
//                             "Error occured while broadcasting Part commitment
// to peers :{:?}",                             e
//                         );
//                     },
//                 }
//             },
//             Event::SyncPeers(peers) => {
//                 let mut quic_addresses = vec![];
//                 let mut raptor_peer_list = vec![];
//                 for peer in peers.iter() {
//                     let addr = peer.address;
//                     quic_addresses.push(addr);
//                     let mut raptor_addr = addr;
//                     raptor_addr.set_port(peer.raptor_udp_port);
//                     raptor_peer_list.push(raptor_addr);
//                 }
//                 self.broadcast_engine.add_raptor_peers(raptor_peer_list);
//                 self.broadcast_engine
//                     .add_peer_connection(quic_addresses)
//                     .await?;
//             },
//             Event::Vote(vote, farmer_quorum_threshold) => {
//                 let status = self
//                     .broadcast_engine
//                     .quic_broadcast(Message::new(MessageBody::Vote {
//                         vote,
//                         farmer_quorum_threshold,
//                     }))
//                     .await;
//                 match status {
//                     Ok(_) => {},
//                     Err(e) => {
//                         error!(
//                             "Error occured while broadcasting votes to
// harvesters :{:?}",                             e
//                         );
//                     },
//                 }
//             },
//             // Broadcasting the Convergence block to the peers.
//             Event::BlockConfirmed(block) => {
//                 let status = self
//                     .broadcast_engine
//                     .unreliable_broadcast(
//                         block,
//                         RAPTOR_ERASURE_COUNT,
//                         self.broadcast_engine.raptor_udp_port,
//                     )
//                     .await;
//                 match status {
//                     Ok(_) => {},
//                     Err(e) => {
//                         error!("Error occured while broadcasting blocks to
// peers :{:?}", e);                     },
//                 }
//             },
//             Event::ForwardTxn((txn_record, addresses)) => {
//                 for address in addresses.iter() {
//                     let address = *address;
//                     let status = self
//                         .broadcast_engine
//                         .send_data_via_quic(
//                             
// Message::new(MessageBody::ForwardedTxn(txn_record.clone())),                 
// address,                         )
//                         .await;
//                     match status {
//                         Ok(_) => {},
//                         Err(e) => {
//                             error!(
//                                 "Error occurred while forwarding transaction
// to peers: {:?}",                                 e
//                             );
//                         },
//                     }
//                 }
//             },
//
//             _ => {},
//         }
//
//         Ok(ActorState::Running)
//     }
// }
//
// #[cfg(test)]
// mod tests {
//
//     use events::{Event, EventMessage, SyncPeerData, DEFAULT_BUFFER};
//     use primitives::NodeType;
//     use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
//     use theater::{Actor, ActorImpl};
//     use tokio::net::UdpSocket;
//
//     use super::{BroadcastModule, BroadcastModuleConfig};
//
//     #[tokio::test]
//     async fn test_broadcast_module() {
//         let (internal_events_tx, mut internal_events_rx) =
//             tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
//
//         let node_id = uuid::Uuid::new_v4().to_string().into_bytes();
//
//         let mut db_config = VrrbDbConfig::default();
//
//         let temp_dir_path = std::env::temp_dir();
//         let db_path =
// temp_dir_path.join(vrrb_core::helpers::generate_random_string());
//
//         db_config.with_path(db_path);
//
//         let db = VrrbDb::new(db_config);
//
//         let vrrbdb_read_handle = db.read_handle();
//
//         let config = BroadcastModuleConfig {
//             events_tx: internal_events_tx,
//             vrrbdb_read_handle,
//             node_type: NodeType::Full,
//             udp_gossip_address_port: 0,
//             raptorq_gossip_address_port: 0,
//             node_id,
//         };
//
//         let (events_tx, mut events_rx) =
//             tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
//
//         let broadcast_module = BroadcastModule::new(config).await.unwrap();
//
//         let mut broadcast_module_actor = ActorImpl::new(broadcast_module);
//
//         let handle = tokio::spawn(async move {
//             broadcast_module_actor.start(&mut events_rx).await.unwrap();
//         });
//
//         let bound_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
//
//         let address = bound_socket.local_addr().unwrap();
//
//         let peer_data = SyncPeerData {
//             address,
//             raptor_udp_port: 9993,
//             quic_port: 9994,
//             node_type: NodeType::Full,
//         };
//
//         events_tx
//             .send(Event::SyncPeers(vec![peer_data.clone()]).into())
//             .unwrap();
//
//         events_tx.send(Event::Stop.into()).unwrap();
//
//         match internal_events_rx.recv().await {
//             Some(value) => assert_eq!(value,
// Event::SyncPeers(vec![peer_data]).into()),             None => {},
//         }
//
//         handle.await.unwrap();
//     }
// }
