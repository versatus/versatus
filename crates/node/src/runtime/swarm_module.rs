use std::{hash::Hash, net::SocketAddr, path::PathBuf};

use async_trait::async_trait;
use kademlia_dht::{Key, Node, NodeData};
use lr_trie::ReadHandleFactory;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use state::{NodeState, NodeStateConfig, NodeStateReadHandle};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use tracing::error;
use vrrb_core::event_router::{DirectedEvent, Event, Topic};
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
    pub node: Node,
    is_bootstrap_node: bool,
    refresh_interval: Option<u64>,
    ping_interval: Option<u64>,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}


impl SwarmModule {
    pub fn new(
        config: SwarmModuleConfig,
        refresh_interval: Option<u64>,
        ping_interval: Option<u64>,
        events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    ) -> Self {
        let mut is_bootstrap_node = false;
        let node = if let Some(bootstrap_node) = config.bootstrap_node {
            let key_bytes = hex::decode(bootstrap_node.key).unwrap();
            let key: kademlia_dht::Key = Key::try_from(key_bytes).unwrap();
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
            Node::new(
                "127.0.0.1",
                config.port.to_string().as_str(),
                Some(bootstrap_node_data),
            )
        } else {
            //boostrap Node creation
            Node::new("127.0.0.1", config.port.to_string().as_str(), None)
        };
        Self {
            node,
            is_bootstrap_node,
            refresh_interval,
            ping_interval,
            events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
        }
    }

    fn name(&self) -> String {
        String::from("Swarm module")
    }
}

#[async_trait]
impl Handler<Event> for SwarmModule {
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
                self.node.kill();
                return Ok(ActorState::Stopped);
            },
            Event::NoOp => {},
            _ => telemetry::warn!("Unrecognized command received: {:?}", event),
        }

        Ok(ActorState::Running)
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

    use primitives::NodeType;
    use theater::ActorImpl;
    use udp2p::protocol::protocol::Peer;
    use vrrb_core::{
        event_router::{DirectedEvent, Event, PeerData},
        txn::NULL_TXN,
    };

    use super::*;

    #[tokio::test]
    async fn swarm_runtime_module_starts_and_stops() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();

        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6060,
                bootstrap_node: None,
            },
            None,
            None,
            events_tx,
        );
        let mut swarm_module = ActorImpl::new(bootstrap_swarm_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(swarm_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn swarm_runtime_add_peers() {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut bootstrap_swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6061,
                bootstrap_node: None,
            },
            None,
            None,
            events_tx,
        );
        let key = bootstrap_swarm_module.node.node_data().id.0.to_vec();
        let (ctrl_boot_strap_tx, mut ctrl_boot_strap_rx) =
            tokio::sync::broadcast::channel::<Event>(10);
        assert_eq!(bootstrap_swarm_module.status(), ActorState::Stopped);


        let (events_node_tx, events_node_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut swarm_module = SwarmModule::new(
            SwarmModuleConfig {
                port: 6062,
                bootstrap_node: Some(BootStrapNodeDetails {
                    addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6061),
                    key: hex::encode(key),
                }),
            },
            None,
            None,
            events_node_tx,
        );

        let node_key = swarm_module.node.node_data().id.0.clone();

        let current_node_id = swarm_module.node.node_data().id.clone();
        let mut swarm_module = ActorImpl::new(swarm_module);
        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);
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
        assert_eq!(nodes.get(0).unwrap().id, current_node_id);
        ctrl_tx.send(Event::Stop).unwrap();
        handle.await.unwrap();
    }
}
