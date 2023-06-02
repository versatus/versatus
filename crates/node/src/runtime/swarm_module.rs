use std::net::SocketAddr;

use async_trait::async_trait;
use events::{Event, EventMessage, EventPublisher};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler};

use crate::{result::Result, NodeError};

type Port = usize;

#[derive(Clone)]
pub struct SwarmModuleConfig {
    pub port: Port,
    pub bootstrap_node: Option<BootStrapNodeDetails>,
}

#[derive(Clone)]
pub struct BootStrapNodeDetails {
    pub addr: SocketAddr,
    pub key: String,
}

#[derive(Clone)]
pub struct SwarmModule {
    pub node: KademliaNode,
    is_bootstrap_node: bool,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
}

impl SwarmModule {
    pub fn new(config: SwarmModuleConfig, events_tx: EventPublisher) -> Result<Self> {
        let node = if let Some(bootstrap_node) = config.bootstrap_node.clone() {
            let key_bytes = hex::decode(&bootstrap_node.key).map_err(|_| {
                NodeError::Other(String::from(
                    "Invalid Hex string key for bootstrap_node key",
                ))
            })?;
            let key = Key::try_from(key_bytes).map_err(|_| {
                NodeError::Other(String::from(
                    "Invalid Node Key, Node Key should be 32 bytes",
                ))
            })?;
            let bootstrap_node_data = NodeData::new(
                bootstrap_node.addr.ip().to_string(),
                bootstrap_node.addr.port().to_string(),
                format!(
                    "{}:{}",
                    bootstrap_node.addr.ip(),
                    bootstrap_node.addr.port()
                ),
                key,
            );

            KademliaNode::new(
                "127.0.0.1",
                &config.port.to_string(),
                Some(bootstrap_node_data),
            )
        } else {
            KademliaNode::new("127.0.0.1", &config.port.to_string(), None)
        };

        Ok(Self {
            node,
            is_bootstrap_node: config.bootstrap_node.is_none(),
            events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
        })
    }

    fn name(&self) -> String {
        String::from("Swarm module")
    }
}

#[async_trait]
impl Handler<EventMessage> for SwarmModule {
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

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                self.node.kill();
                return Ok(ActorState::Stopped);
            },
            Event::NoOp => {},
            Event::FetchPeers(no) => {
                let key = self.node.node_data().id.clone();
                let closest_nodes = self
                    .node
                    .routing_table
                    .lock()
                    .unwrap()
                    .get_closest_nodes(&key, no);

                for node in closest_nodes {
                    println!("Closest Node with Key : {:?} :{:?}", key, node);
                }
            },
            Event::DHTStoreRequest(key, value) => {
                info!(
                    "Storing into DHT Store Request  :{:?}:{:?}",
                    KademliaNode::get_key(key.as_str()),
                    value
                );
                self.node
                    .insert(KademliaNode::get_key(key.as_str()), value.as_str());
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        thread,
        time::Duration,
    };

    use events::{Event, EventMessage, DEFAULT_BUFFER};
    use serial_test::serial;
    use theater::ActorImpl;
    use tokio::sync::broadcast::{Receiver, Sender};

    use super::*;

    #[tokio::test]
    #[serial]
    async fn swarm_runtime_module_starts_and_stops() {
        let (events_tx, _) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 0,
                bootstrap_node: None,
            },
            events_tx,
        )
        .unwrap();
        let mut swarm_module = ActorImpl::new(bootstrap_swarm_module);

        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);

        assert_eq!(swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(swarm_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }


    #[tokio::test]
    #[serial]
    async fn swarm_runtime_add_peers() {
        let (events_tx, _events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6061,
                bootstrap_node: None,
            },
            events_tx.clone(),
        )
        .unwrap();

        let key = bootstrap_swarm_module.node.node_data().id.0.to_vec();
        let mut handles = Vec::new();
        let mut ctrl_txs = Vec::new();
        for port in 6062..=6070 {
            let (ctrl_tx, mut ctrl_rx) =
                tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
            let swarm_module = SwarmModule::new(
                SwarmModuleConfig {
                    port,
                    bootstrap_node: Some(BootStrapNodeDetails {
                        addr: SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                            bootstrap_swarm_module
                                .node
                                .node_data()
                                .port
                                .parse()
                                .unwrap(),
                        ),
                        key: hex::encode(&key),
                    }),
                },
                events_tx.clone(),
            )
            .unwrap();
            let handle = start_swarm_module(swarm_module, ctrl_rx).await;
            handles.push(handle);
            ctrl_txs.push(ctrl_tx);
        }
        for ctrl_tx in ctrl_txs.iter() {
            ctrl_tx.send(Event::FetchPeers(3).into()).unwrap();
        }
        for ctrl_tx in ctrl_txs.iter() {
            ctrl_tx.send(Event::Stop.into()).unwrap();
        }


        for handle in handles {
            handle.await.unwrap();
        }
    }

    async fn start_swarm_module(
        swarm_module: SwarmModule,
        mut ctrl_rx: Receiver<EventMessage>,
    ) -> tokio::task::JoinHandle<()> {
        let mut actor_impl = ActorImpl::new(swarm_module);
        assert_eq!(actor_impl.status(), ActorState::Stopped);
        tokio::spawn(async move {
            actor_impl.start(&mut ctrl_rx).await.unwrap();
        })
    }
    #[tokio::test]
    #[serial]
    async fn swarm_runtime_fetch_data_from_dht_peers() {
        let (events_tx, _events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6061,
                bootstrap_node: None,
            },
            events_tx.clone(),
        )
        .unwrap();

        let key = bootstrap_swarm_module.node.node_data().id.0.to_vec();
        let mut handles = Vec::new();
        let mut ctrl_txs = Vec::new();
        let mut swarm_nodes = vec![];
        for port in 6062..=6065 {
            let (ctrl_tx, mut ctrl_rx) =
                tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
            let swarm_module = SwarmModule::new(
                SwarmModuleConfig {
                    port,
                    bootstrap_node: Some(BootStrapNodeDetails {
                        addr: SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                            bootstrap_swarm_module
                                .node
                                .node_data()
                                .port
                                .parse()
                                .unwrap(),
                        ),
                        key: hex::encode(&key),
                    }),
                },
                events_tx.clone(),
            )
            .unwrap();
            if port == 6062 {
                let handle = start_swarm_module(swarm_module, ctrl_rx).await;
                handles.push(handle);
                ctrl_txs.push(ctrl_tx);
            } else {
                swarm_nodes.push(swarm_module);
            }
        }
        if let Some(ctrl_tx) = ctrl_txs.get(0) {
            ctrl_tx
                .send(Event::DHTStoreRequest(String::from("Hello"), String::from("Vrrb")).into())
                .unwrap();
        }

        thread::sleep(Duration::from_secs(2));

        for ctrl_tx in ctrl_txs.iter() {
            ctrl_tx.send(Event::Stop.into()).unwrap();
        }


        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(
            swarm_nodes
                .get_mut(0)
                .unwrap()
                .node
                .get(&KademliaNode::get_key("Hello")),
            Some("Vrrb")
        );
        assert_eq!(
            swarm_nodes
                .get_mut(1)
                .unwrap()
                .node
                .get(&KademliaNode::get_key("Hello")),
            Some("Vrrb")
        );
        assert_eq!(
            swarm_nodes
                .get_mut(2)
                .unwrap()
                .node
                .get(&KademliaNode::get_key("Hello")),
            Some("Vrrb")
        );
    }


    #[tokio::test]
    #[serial]
    async fn swarm_runtime_test_unreachable_peers() {
        let (events_tx, _events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 0,
                bootstrap_node: None,
            },
            events_tx,
        )
        .unwrap();
        let key = bootstrap_swarm_module.node.node_data().id.0.to_vec();
        let (_ctrl_boot_strap_tx, _ctrl_boot_strap_rx) =
            tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(bootstrap_swarm_module.status(), ActorState::Stopped);

        let (events_node_tx, _events_node_rx) =
            tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 0,
                bootstrap_node: Some(BootStrapNodeDetails {
                    addr: SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        bootstrap_swarm_module
                            .node
                            .node_data()
                            .port
                            .parse()
                            .unwrap(),
                    ),
                    key: hex::encode(key.clone()),
                }),
            },
            events_node_tx,
        )
        .unwrap();

        let current_node_id = swarm_module.node.node_data().id;
        let target_port = swarm_module.node.node_data().port;

        let mut swarm_module = ActorImpl::new(swarm_module);
        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
        assert_eq!(swarm_module.status(), ActorState::Stopped);
        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
        });

        let _s = bootstrap_swarm_module.node.rpc_ping(&NodeData {
            ip: "127.0.0.1".to_string(),
            port: target_port.clone(),
            addr: "127.0.0.1".to_string() + &*target_port,
            id: current_node_id,
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();

        let s = bootstrap_swarm_module.node.rpc_ping(&NodeData {
            ip: "127.0.0.1".to_string(),
            port: "6064".to_string(),
            addr: "127.0.0.1:6064".to_string(),
            id: current_node_id,
        });

        assert!(s.is_none());
    }
}
