use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs,
    io::{Read, Write},
    net::SocketAddr,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use block::invalid::InvalidBlockErrorReason;
use kademlia_dht::Node;
use miner::miner::Miner;
use network::{components::StateComponent, message};
use public_ip;
use rand::{thread_rng, Rng};
use reward::reward::Reward;
use ritelinked::LinkedHashMap;
use state::{Components, NetworkState};
use telemetry::info;
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};
use wallet::wallet::WalletAccount;

use crate::{node_auth::NodeAuth, result::Result, RuntimeModule, RuntimeModuleState, StateModule};

type Port = usize;

#[derive(Debug)]
pub struct SwarmConfig {
    pub port: Port,
    pub bootstrap_node: Option<SocketAddr>,
}

#[derive(Clone)]
pub struct SwarmModule {
    node: Node,
    refresh_interval: Option<u64>,
    ping_interval: Option<u64>,
    running_status: RuntimeModuleState,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

#[async_trait]
impl RuntimeModule for SwarmModule {
    fn name(&self) -> String {
        String::from("Swarm module")
    }

    fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    async fn start(
        &mut self,
        events_rx: &mut tokio::sync::broadcast::Receiver<Event>,
    ) -> Result<()> {
        loop {
            let event = self.decode_event(events_rx.try_recv());
            if event == Event::Stop {
                telemetry::info!("{0} received stop signal. Stopping", self.name());
                self.running_status = RuntimeModuleState::Terminating;
                self.node.kill();
                break;
            }
            self.process_event(event);
        }

        self.running_status = RuntimeModuleState::Stopped;

        Ok(())
    }
}


impl SwarmModule {
    fn decode_event(&mut self, event: std::result::Result<Event, TryRecvError>) -> Event {
        match event {
            Ok(cmd) => cmd,
            Err(err) => match err {
                TryRecvError::Closed => {
                    telemetry::error!("The events channel for event router has been closed.");
                    Event::Stop
                },
                TryRecvError::Lagged(u64) => {
                    telemetry::error!("Receiver lagged behind");
                    Event::NoOp
                },
                _ => Event::NoOp,
            },
            _ => Event::NoOp,
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::AddPeer(peer_info) => {
                telemetry::info!("Adding Peer to Node");
                self.events_tx
                    .send((Topic::Peers, Event::AddPeer(peer_info)))
                    .unwrap();
            },
            Event::RemovePeer(peer_info) => {
                telemetry::info!("Removing Peer from Node");
                self.events_tx
                    .send((Topic::Peers, Event::RemovePeer(peer_info)))
                    .unwrap();
            },
            Event::NoOp => {},
            _ => telemetry::warn!("Unrecognized command received: {:?}", event),
        }
    }

    pub fn new(
        swarm_config: SwarmConfig,
        refresh_interval: Option<u64>,
        ping_interval: Option<u64>,
        events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    ) -> Self {
        let node = if let Some(bootstrap_node) = swarm_config.bootstrap_node {
            let bootstrap_node = Node::new(
                bootstrap_node.ip().to_string().as_str(),
                bootstrap_node.port().to_string().as_str(),
                None,
            );
            Node::new(
                bootstrap_node.node_data().ip.as_str(),
                swarm_config.port.to_string().as_str(),
                Some(bootstrap_node.node_data()),
            )
        } else {
            //boostrap Node creation
            Node::new("localhost", swarm_config.port.to_string().as_str(), None)
        };
        SwarmModule {
            node,
            refresh_interval,
            ping_interval,
            running_status: RuntimeModuleState::Stopped,
            events_tx,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{
        env,
        io,
        mem::swap,
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
        os,
        path::PathBuf,
        rc::Rc,
        sync::Arc,
    };

    use state::node_state;
    use telemetry::TelemetrySubscriber;
    use uuid::Uuid;
    use vrrb_config::NodeConfig;
    use vrrb_core::event_router::{DirectedEvent, Event, EventRouter, PeerInfo, Topic};

    use super::*;

    #[tokio::test]
    async fn swarm_runtime_module_starts_and_stops() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut swarm_module = SwarmModule::new(
            SwarmConfig {
                port: 8090,
                bootstrap_node: None,
            },
            None,
            None,
            events_tx,
        );

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(swarm_module.status(), RuntimeModuleState::Stopped);
        assert!(!swarm_module.node.node_data().addr.is_empty());
        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(swarm_module.status(), RuntimeModuleState::Stopped);
        });

        ctrl_tx.send(Event::Stop).unwrap();
        handle.await.unwrap();
    }


    #[tokio::test]
    async fn swarm_runtime_publish_peer_event() {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmConfig {
                port: 8080,
                bootstrap_node: None,
            },
            None,
            None,
            events_tx,
        );
        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(bootstrap_swarm_module.status(), RuntimeModuleState::Stopped);
        let routing_table = bootstrap_swarm_module.node.routing_table.clone();
        let bootstrap_node_key = bootstrap_swarm_module.node.node_data().clone();
        let events_handle = tokio::spawn(async move {
            loop {
                match events_rx.recv().await {
                    Some(event) => {
                        if let Topic::Peers = event.0 {
                            if let Event::AddPeer(peer_info_bytes) = event.1 {
                                let (events_tx, _) =
                                    tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
                                let (ctrl_tx, mut ctrl_rx) =
                                    tokio::sync::broadcast::channel::<Event>(10);
                                let mut peer_info: PeerInfo =
                                    serde_json::from_slice(&peer_info_bytes).unwrap();
                                let mut new_node = SwarmModule::new(
                                    SwarmConfig {
                                        port: 8081,
                                        bootstrap_node: Some(peer_info.peer_address),
                                    },
                                    None,
                                    None,
                                    events_tx.clone(),
                                );
                                assert_eq!(
                                    new_node.node.routing_table.lock().unwrap().total_peers(),
                                    1
                                );
                            } else if let Event::RemovePeer(peer_info_bytes) = event.1 {
                                let (events_tx, _) =
                                    tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
                                let (ctrl_tx, mut ctrl_rx) =
                                    tokio::sync::broadcast::channel::<Event>(10);
                                let mut peer_info: PeerInfo =
                                    serde_json::from_slice(&peer_info_bytes).unwrap();
                                let mut new_node = SwarmModule::new(
                                    SwarmConfig {
                                        port: 8081,
                                        bootstrap_node: Some(peer_info.peer_address),
                                    },
                                    None,
                                    None,
                                    events_tx.clone(),
                                );
                                new_node
                                    .node
                                    .routing_table
                                    .lock()
                                    .unwrap()
                                    .update_node(bootstrap_node_key.clone());
                                assert_eq!(
                                    new_node.node.routing_table.lock().unwrap().total_peers(),
                                    2
                                );
                                new_node
                                    .node
                                    .routing_table
                                    .lock()
                                    .unwrap()
                                    .remove_node(&bootstrap_node_key);
                                assert_eq!(
                                    new_node.node.routing_table.lock().unwrap().total_peers(),
                                    1
                                );
                            }
                        }
                    },
                    None => {
                        // println!("No  occurred here");
                        break;
                    },
                }
            }
        });
        let key = bootstrap_swarm_module.node.node_data().id;
        let handle = tokio::spawn(async move {
            bootstrap_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(bootstrap_swarm_module.status(), RuntimeModuleState::Stopped);
        });

        let node_a_peer_address = SocketAddr::new(
            std::net::IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            8080,
        );

        let peer_info = PeerInfo::new(node_a_peer_address, 0);
        let serialized_peer_info = serde_json::to_string(&peer_info)
            .unwrap()
            .as_bytes()
            .to_vec();

        ctrl_tx
            .send(Event::RemovePeer(serialized_peer_info))
            .unwrap();
        thread::sleep(Duration::from_secs(1));
        ctrl_tx.send(Event::Stop).unwrap();
        handle.await.unwrap();
        events_handle.await.unwrap();
    }
}
