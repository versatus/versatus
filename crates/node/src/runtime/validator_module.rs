use std::path::PathBuf;

use crate::{result::Result, NodeError, RuntimeModule, RuntimeModuleState};
use state::{state::NetworkState, NodeState};
use tokio::sync::mpsc::error::TryRecvError;
use validator::validator_unit::ValidatorUnit;
use vrrb_core::event_router::{Event, Topic};

pub struct ValidatorModule {
    running_status: RuntimeModuleState,
}

/// ValidatorModule manages all validation tasks within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled as necessary.
impl ValidatorModule {
    pub fn new() -> Self {
        Self {
            // validator: ValidatorUnit::new(),
            running_status: RuntimeModuleState::Stopped,
        }
    }
}

// impl StateModule {
//     fn decode_event(&mut self, event: std::result::Result<Event, TryRecvError>) -> Event {
//         match event {
//             Ok(cmd) => cmd,
//             Err(err) if err == TryRecvError::Disconnected => {
//                 telemetry::error!("The events channel for event router has been closed.");
//                 Event::Stop
//             },
//             _ => Event::NoOp,
//         }
//     }
//
//     fn process_event(&mut self, event: Event) {
//         match event {
//             Event::TxnCreated(_) => {
//                 telemetry::info!("Storing transaction in tx tree.");
//             },
//             _ => telemetry::warn!("Unrecognized command received: {:?}", event),
//         }
//     }
// }
//
// impl RuntimeModule for StateModule {
//     fn name(&self) -> String {
//         String::from("State module")
//     }
//
//     fn status(&self) -> RuntimeModuleState {
//         self.running_status.clone()
//     }
//
//     fn start(
//         &mut self,
//         event_stream: &mut tokio::sync::mpsc::UnboundedReceiver<Event>,
//     ) -> Result<()> {
//         loop {
//             let event = self.decode_event(event_stream.try_recv());
//
//             if event == Event::Stop {
//                 telemetry::info!("{0} received stop signal. Stopping", self.name());
//
//                 self.running_status = RuntimeModuleState::Terminating;
//                 // TODO: do cleanup work like backing up the state
//
//                 self.state
//                     .serialize_to_json()
//                     .map_err(|err| NodeError::Other(err.to_string()))?;
//
//                 break;
//             }
//
//             self.process_event(event);
//         }
//
//         self.running_status = RuntimeModuleState::Stopped;
//
//         Ok(())
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use std::{
//         env, io,
//         net::{IpAddr, Ipv4Addr, SocketAddr},
//         os,
//         path::PathBuf,
//         rc::Rc,
//         sync::Arc,
//     };
//
//     use super::*;
//     use commands::command::Command;
//     use state::node_state;
//     use telemetry::TelemetrySubscriber;
//     use uuid::Uuid;
//     use vrrb_config::NodeConfig;
//     use vrrb_core::event_router::{DirectedEvent, Event, EventRouter, Topic};
//
//     #[tokio::test]
//     async fn state_runtime_module_starts_and_stops() {
//         let temp_dir_path = env::temp_dir();
//         let mut state_path = temp_dir_path.clone().join("state.json");
//
//         let mut state_module = StateModule::new(state_path);
//
//         let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
//
//         assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
//
//         let handle = tokio::spawn(async move {
//             state_module.start(&mut ctrl_rx).unwrap();
//             assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
//         });
//
//         ctrl_tx.send(Event::Stop).unwrap();
//
//         handle.await.unwrap();
//     }
//
//     #[tokio::test]
//     async fn state_runtime_receives_new_txn_event() {
//         let temp_dir_path = env::temp_dir();
//         let mut state_path = temp_dir_path.clone().join("state.json");
//
//         let mut state_module = StateModule::new(state_path);
//
//         let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
//
//         assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
//
//         let handle = tokio::spawn(async move {
//             state_module.start(&mut ctrl_rx).unwrap();
//             assert_eq!(state_module.status(), RuntimeModuleState::Stopped);
//         });
//
//         ctrl_tx.send(Event::TxnCreated(vec![])).unwrap();
//
//         ctrl_tx.send(Event::Stop).unwrap();
//
//         handle.await.unwrap();
//     }
// }
//
// /*
// pub fn state_sending_thread() {
//     let state_to_swarm_sender = to_swarm_sender.clone();
//     let state_to_gossip_sender = to_gossip_tx.clone();
//     let state_to_blockchain_sender = to_blockchain_sender.clone();
//     let state_node_id = node_id.clone();
//     std::thread::spawn(move || loop {
//         let blockchain_sender = state_to_blockchain_sender.clone();
//         let swarm_sender = state_to_swarm_sender.clone();
//         let gossip_sender = state_to_gossip_sender.clone();
//         if let Ok(command) = to_state_receiver.try_recv() {
//             match command {
//                 Command::SendStateComponents(requestor, component_bytes, sender_id) => {
//                     let command =
//                         Command::GetStateComponents(requestor, component_bytes, sender_id);
//                     if let Err(e) = blockchain_sender.send(command) {
//                         info!(
//                             "Error sending component request to blockchain thread: {:?}",
//                             e
//                         );
//                     }
//                 },
//                 Command::StoreStateComponents(data, component_type) => {
//                     if let Err(e) =
//                         blockchain_sender.send(Command::StoreStateComponents(data, component_type))
//                     {
//                         info!("Error sending component to blockchain")
//                     }
//                 },
//                 Command::RequestedComponents(requestor, components, sender_id, requestor_id) => {
//                     let restructured_components = Components::from_bytes(&components);
//                     let head = Header::Gossip;
//                     let message = MessageType::StateComponentsMessage {
//                         data: restructured_components.as_bytes(),
//                         requestor: requestor.clone(),
//                         requestor_id,
//                         sender_id,
//                     };
//
//                     let msg_id = MessageKey::rand();
//                     let gossip_msg = GossipMessage {
//                         id: msg_id.inner(),
//                         data: message.as_bytes(),
//                         sender: addr.clone(),
//                     };
//
//                     let msg = Message {
//                         head,
//                         msg: gossip_msg.as_bytes().unwrap(),
//                     };
//
//                     let requestor_addr: SocketAddr =
//                         requestor.parse().expect("Unable to parse address");
//
//                     match requestor_addr {
//                         SocketAddr::V4(v4) => {
//                             info!("Requestor is a v4 IP");
//                             let ip = v4.ip().clone();
//                             let port = 19291;
//                             let new_addr = SocketAddrV4::new(ip, port);
//                             let tcp_addr = SocketAddr::from(new_addr);
//                             match std::net::TcpStream::connect(new_addr) {
//                                 Ok(mut stream) => {
//                                     info!("Opened TCP stream and connected to requestor");
//                                     let msg_bytes = msg.as_bytes().unwrap();
//                                     let n_bytes = msg_bytes.len();
//                                     stream.write(&msg_bytes).unwrap();
//                                     info!("Wrote {:?} bytes to tcp stream for requestor", n_bytes);
//                                     stream
//                                         .shutdown(std::net::Shutdown::Both)
//                                         .expect("Unable to shutdown");
//                                 },
//                                 Err(_) => {},
//                             }
//                         },
//                         SocketAddr::V6(v6) => {},
//                     }
//                 },
//                 _ => {
//                     info!("Received State Command: {:?}", command);
//                 },
//             }
//         }
//     });
// }
// */
