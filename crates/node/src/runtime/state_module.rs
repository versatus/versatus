use state::state::NetworkState;
use tokio::sync::mpsc::error::TryRecvError;
use vrrb_core::event_router::Event;


use crate::{result::Result, RuntimeModule, RuntimeModuleState};

pub struct StateModule {
    //
}

/// StateModule manages all state persistence and updates within VrrbNodes
/// it runs as an indepdendant module such that it can be enabled and disabled as necessary.
impl StateModule {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuntimeModule for StateModule {
    fn name(&self) -> String {
        String::from("State module")
    }

    fn status(&self) -> RuntimeModuleState {
        todo!()
    }

    fn start(
        &mut self,
        control_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Event>,
    ) -> Result<()> {
        // TODO: rethink this loop
        loop {
            match control_rx.try_recv() {
                Ok(sig) => {
                    telemetry::info!("Received stop signal");
                    break;
                },
                Err(err) if err == TryRecvError::Disconnected => {
                    telemetry::warn!("Failed to process stop signal. Reason: {0}", err);
                    telemetry::warn!("{} shutting down", self.name());
                    break;
                },
                _ => {},
            }
        }

        Ok(())
    }
    //
}

/*
pub fn state_sending_thread() {
    let state_to_swarm_sender = to_swarm_sender.clone();
    let state_to_gossip_sender = to_gossip_tx.clone();
    let state_to_blockchain_sender = to_blockchain_sender.clone();
    let state_node_id = node_id.clone();
    std::thread::spawn(move || loop {
        let blockchain_sender = state_to_blockchain_sender.clone();
        let swarm_sender = state_to_swarm_sender.clone();
        let gossip_sender = state_to_gossip_sender.clone();
        if let Ok(command) = to_state_receiver.try_recv() {
            match command {
                Command::SendStateComponents(requestor, component_bytes, sender_id) => {
                    let command =
                        Command::GetStateComponents(requestor, component_bytes, sender_id);
                    if let Err(e) = blockchain_sender.send(command) {
                        info!(
                            "Error sending component request to blockchain thread: {:?}",
                            e
                        );
                    }
                },
                Command::StoreStateComponents(data, component_type) => {
                    if let Err(e) =
                        blockchain_sender.send(Command::StoreStateComponents(data, component_type))
                    {
                        info!("Error sending component to blockchain")
                    }
                },
                Command::RequestedComponents(requestor, components, sender_id, requestor_id) => {
                    let restructured_components = Components::from_bytes(&components);
                    let head = Header::Gossip;
                    let message = MessageType::StateComponentsMessage {
                        data: restructured_components.as_bytes(),
                        requestor: requestor.clone(),
                        requestor_id,
                        sender_id,
                    };

                    let msg_id = MessageKey::rand();
                    let gossip_msg = GossipMessage {
                        id: msg_id.inner(),
                        data: message.as_bytes(),
                        sender: addr.clone(),
                    };

                    let msg = Message {
                        head,
                        msg: gossip_msg.as_bytes().unwrap(),
                    };

                    let requestor_addr: SocketAddr =
                        requestor.parse().expect("Unable to parse address");

                    match requestor_addr {
                        SocketAddr::V4(v4) => {
                            info!("Requestor is a v4 IP");
                            let ip = v4.ip().clone();
                            let port = 19291;
                            let new_addr = SocketAddrV4::new(ip, port);
                            let tcp_addr = SocketAddr::from(new_addr);
                            match std::net::TcpStream::connect(new_addr) {
                                Ok(mut stream) => {
                                    info!("Opened TCP stream and connected to requestor");
                                    let msg_bytes = msg.as_bytes().unwrap();
                                    let n_bytes = msg_bytes.len();
                                    stream.write(&msg_bytes).unwrap();
                                    info!("Wrote {:?} bytes to tcp stream for requestor", n_bytes);
                                    stream
                                        .shutdown(std::net::Shutdown::Both)
                                        .expect("Unable to shutdown");
                                },
                                Err(_) => {},
                            }
                        },
                        SocketAddr::V6(v6) => {},
                    }
                },
                _ => {
                    info!("Received State Command: {:?}", command);
                },
            }
        }
    });
}
*/
