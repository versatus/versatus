use std::net::SocketAddr;

use async_trait::async_trait;
use events::{Event, EventMessage, EventPublisher};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler};

use crate::{result::Result, NodeError};

type Port = usize;

#[derive(Debug, Clone)]
pub struct SwarmModuleConfig {
    pub addr: SocketAddr,
    pub bootstrap_node_config: Option<BootstrapNodeConfig>,
}

#[derive(Debug, Clone)]
pub struct BootstrapNodeConfig {
    pub addr: SocketAddr,
    pub key: String,
}

#[derive(Clone)]
pub struct SwarmModule {
    kademlia_node: KademliaNode,
    id: ActorId,
    label: ActorLabel,
    status: ActorState,
    events_tx: EventPublisher,
}

impl SwarmModule {
    pub fn new(config: SwarmModuleConfig, events_tx: EventPublisher) -> Result<Self> {
        let kademlia_node = if let Some(bootstrap_node_config) = config.bootstrap_node_config {
            let node_key_bytes = hex::decode(bootstrap_node_config.key).map_err(|err| {
                NodeError::Other(format!(
                    "Invalid hex string key for boostrap node key: {err}",
                ))
            })?;

            let kademlia_key = Key::try_from(node_key_bytes).map_err(|err| {
                NodeError::Other(format!("Node key should have a 32 byte length: {err}"))
            })?;

            let bootstrap_node_addr_str = format!(
                "{}:{}",
                bootstrap_node_config.addr.ip(),
                bootstrap_node_config.addr.port(),
            );

            // TODO: figure out why kademlia_dht needs the ip, port and then the whole
            // address separately
            // NOTE: this snippet turns the bootstrap node config into a NodeData struct
            // that kademlia_dht understands
            let bootstrap_node_data = NodeData::new(
                bootstrap_node_config.addr.ip().to_string(),
                bootstrap_node_config.addr.port().to_string(),
                bootstrap_node_addr_str,
                kademlia_key,
            );

            KademliaNode::new(
                config.addr.ip().to_string().as_str(),
                config.addr.port().to_string().as_str(),
                Some(bootstrap_node_data),
            )
        } else {
            // NOTE: become a bootstrap node if none is provided
            KademliaNode::new(
                config.addr.ip().to_string().as_str(),
                config.addr.port().to_string().as_str(),
                None,
            )
        };

        Ok(Self {
            kademlia_node,
            events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
        })
    }

    pub fn node_ref(&self) -> &KademliaNode {
        &self.kademlia_node
    }

    pub fn node_mut(&mut self) -> &mut KademliaNode {
        &mut self.kademlia_node
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
            Event::FetchPeers(no) => {
                let key = self.kademlia_node.node_data().id.clone();
                let closest_nodes = self
                    .kademlia_node
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
                self.kademlia_node
                    .insert(KademliaNode::get_key(key.as_str()), value.as_str());
            },
            Event::Stop => {
                self.node_ref().kill();
                return Ok(ActorState::Stopped);
            },
            Event::NoOp => {},
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
                addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                bootstrap_node_config: None,
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
                addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                bootstrap_node_config: None,
            },
            events_tx.clone(),
        )
        .unwrap();

        let key = bootstrap_swarm_module
            .kademlia_node
            .node_data()
            .id
            .0
            .to_vec();
        let mut handles = Vec::new();
        let mut ctrl_txs = Vec::new();
        for port in 6062..=6070 {
            let (ctrl_tx, mut ctrl_rx) =
                tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
            let swarm_module = SwarmModule::new(
                SwarmModuleConfig {
                    addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                    bootstrap_node_config: Some(BootstrapNodeConfig {
                        addr: SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                            bootstrap_swarm_module
                                .kademlia_node
                                .node_data()
                                .port
                                .parse()
                                .unwrap(),
                        ),
                        key: hex::encode(key.clone()),
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
                addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                bootstrap_node_config: None,
            },
            events_tx.clone(),
        )
        .unwrap();

        let key = bootstrap_swarm_module
            .kademlia_node
            .node_data()
            .id
            .0
            .to_vec();
        let mut handles = Vec::new();
        let mut ctrl_txs = Vec::new();
        let mut swarm_nodes = vec![];
        for port in 6062..=6065 {
            let (ctrl_tx, mut ctrl_rx) =
                tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
            let swarm_module = SwarmModule::new(
                SwarmModuleConfig {
                    addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                    bootstrap_node_config: Some(BootstrapNodeConfig {
                        addr: SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                            bootstrap_swarm_module
                                .kademlia_node
                                .node_data()
                                .port
                                .parse()
                                .unwrap(),
                        ),
                        key: hex::encode(key.clone()),
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
                .kademlia_node
                .get(&KademliaNode::get_key("Hello")),
            Some("Vrrb".to_string())
        );
        assert_eq!(
            swarm_nodes
                .get_mut(1)
                .unwrap()
                .kademlia_node
                .get(&KademliaNode::get_key("Hello")),
            Some("Vrrb".to_string())
        );
        assert_eq!(
            swarm_nodes
                .get_mut(2)
                .unwrap()
                .kademlia_node
                .get(&KademliaNode::get_key("Hello")),
            Some("Vrrb".to_string())
        );
    }


    #[tokio::test]
    #[serial]
    async fn swarm_runtime_test_unreachable_peers() {
        let (events_tx, _events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                bootstrap_node_config: None,
            },
            events_tx.clone(),
        )
        .unwrap();
        let key = bootstrap_swarm_module
            .kademlia_node
            .node_data()
            .id
            .0
            .to_vec();

        let (_ctrl_boot_strap_tx, _ctrl_boot_strap_rx) =
            tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(bootstrap_swarm_module.status(), ActorState::Stopped);

        let (events_node_tx, _events_node_rx) =
            tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                addr: "127.0.0.1:6061".parse::<SocketAddr>().unwrap(),
                bootstrap_node_config: Some(BootstrapNodeConfig {
                    addr: SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        bootstrap_swarm_module
                            .kademlia_node
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

        let current_node_id = swarm_module.kademlia_node.node_data().id;
        let target_port = swarm_module.kademlia_node.node_data().port;

        let mut swarm_module = ActorImpl::new(swarm_module);
        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
        assert_eq!(swarm_module.status(), ActorState::Stopped);
        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
        });

        let _s = bootstrap_swarm_module.kademlia_node.rpc_ping(&NodeData {
            ip: "127.0.0.1".to_string(),
            port: target_port.clone(),
            addr: "127.0.0.1".to_string() + &*target_port,
            id: current_node_id,
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();

        let s = bootstrap_swarm_module.kademlia_node.rpc_ping(&NodeData {
            ip: "127.0.0.1".to_string(),
            port: "6064".to_string(),
            addr: "127.0.0.1:6064".to_string(),
            id: current_node_id,
        });

        assert!(s.is_none());
    }
}
