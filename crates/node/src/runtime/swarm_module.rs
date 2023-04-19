use std::{hash::Hash, net::SocketAddr, path::PathBuf};

use async_trait::async_trait;
use events::{Event, EventMessage, EventPublisher};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use tracing::error;

use crate::{result::Result, NodeError};

type Port = usize;

pub struct SwarmModuleConfig {
    pub port: Port,
    pub bootstrap_node: Option<BootStrapNodeDetails>,
}
pub struct BootStrapNodeDetails {
    pub addr: SocketAddr,
    pub key: String,
}

#[derive(Clone)]
pub struct SwarmModule {
    pub node: KademliaNode,
    is_bootstrap_node: bool,
    refresh_interval: Option<u64>,
    ping_interval: Option<u64>,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
}

impl SwarmModule {
    pub fn new(
        config: SwarmModuleConfig,
        refresh_interval: Option<u64>,
        ping_interval: Option<u64>,
        events_tx: EventPublisher,
    ) -> Result<Self> {
        let mut is_bootstrap_node = false;
        let node = if let Some(bootstrap_node) = config.bootstrap_node {
            match hex::decode(bootstrap_node.key) {
                Ok(key_bytes) => match Key::try_from(key_bytes) {
                    Ok(key) => {
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
                        is_bootstrap_node = true;
                        KademliaNode::new(
                            "127.0.0.1",
                            config.port.to_string().as_str(),
                            Some(bootstrap_node_data),
                        )
                    },
                    Err(_) => {
                        return Err(NodeError::Other(String::from(
                            "Invalid Node Key ,Node Key should be 32bytes",
                        )));
                    },
                },
                Err(_e) => {
                    return Err(NodeError::Other(String::from(
                        "Invalid Hex string key for boostrap_node key",
                    )));
                },
            }
        } else {
            //boostrap Node creation
            KademliaNode::new("127.0.0.1", config.port.to_string().as_str(), None)
        };
        Ok(Self {
            node,
            is_bootstrap_node,
            refresh_interval,
            ping_interval,
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
        env,
        net::{IpAddr, Ipv4Addr},
        thread,
        time::Duration,
    };

    use events::{Event, EventMessage, PeerData, DEFAULT_BUFFER};
    use primitives::NodeType;
    use serial_test::serial;
    use theater::ActorImpl;
    use udp2p::protocol::protocol::Peer;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn swarm_runtime_module_starts_and_stops() {
        let (events_tx, _) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);

        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 0,
                bootstrap_node: None,
            },
            None,
            None,
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
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6061,
                bootstrap_node: None,
            },
            None,
            None,
            events_tx,
        )
        .unwrap();

        let key = bootstrap_swarm_module.node.node_data().id.0.to_vec();
        let (ctrl_boot_strap_tx, mut ctrl_boot_strap_rx) =
            tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(bootstrap_swarm_module.status(), ActorState::Stopped);

        let (events_node_tx, events_node_rx) =
            tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);

        let mut swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6062,
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
                    key: hex::encode(key),
                }),
            },
            None,
            None,
            events_node_tx,
        )
        .unwrap();

        let node_key = swarm_module.node.node_data().id.0.clone();

        let current_node_id = swarm_module.node.node_data().id.clone();
        let mut swarm_module = ActorImpl::new(swarm_module);
        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
        assert_eq!(swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
        });
        let nodes = bootstrap_swarm_module
            .node
            .routing_table
            .lock()
            .unwrap()
            .get_closest_nodes(&bootstrap_swarm_module.node.node_data().id, 3);
        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn swarm_runtime_test_unreachable_peers() {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 0,
                bootstrap_node: None,
            },
            None,
            None,
            events_tx,
        )
        .unwrap();
        let key = bootstrap_swarm_module.node.node_data().id.0.to_vec();
        let (ctrl_boot_strap_tx, ctrl_boot_strap_rx) = tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(bootstrap_swarm_module.status(), ActorState::Stopped);

        let (events_node_tx, events_node_rx) =
            tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let mut swarm_module = SwarmModule::new(
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
            None,
            None,
            events_node_tx,
        )
        .unwrap();

        let current_node_id = swarm_module.node.node_data().id.clone();
        let target_port = swarm_module.node.node_data().port;

        let mut swarm_module = ActorImpl::new(swarm_module);
        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);
        assert_eq!(swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
        });

        let s = bootstrap_swarm_module.node.rpc_ping(&NodeData {
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
