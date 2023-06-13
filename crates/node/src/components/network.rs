use std::net::SocketAddr;

use async_trait::async_trait;
use dyswarm::server::ServerConfig;
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use primitives::{NodeId, NodeType};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use tracing::debug;
use vrrb_config::NodeConfig;

use crate::{result::Result, NodeError, RuntimeComponent, RuntimeComponentHandle};

type Port = usize;

pub struct NetworkModule {
    id: ActorId,
    label: ActorLabel,
    status: ActorState,
    events_tx: EventPublisher,
    kademlia_node: KademliaNode,
    dyswarm_server: dyswarm::server::Server,
    dyswarm_client: dyswarm::client::Client,
}

// TODO: Add broadcast listening and sending capabilities here
// TODO: implmement the consensus module which forms quorums and sends keys and
// votes on elections
//
impl std::fmt::Debug for NetworkModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkModule")
            .field("id", &self.id)
            .field("label", &self.label)
            .field("status", &self.status)
            .field("events_tx", &self.events_tx)
            // TODO: impl Debug on KademliaNode
            // .field("kademlia_node", &self.kademlia_node)
            .field("dyswarm_server", &self.dyswarm_server)
            .finish()
    }
    //
}

#[derive(Debug, Clone)]
pub struct NetworkModuleConfig {
    /// This Node's own gossip address
    pub addr: SocketAddr,
    /// Configuration used to connect to a bootstrap node
    pub bootstrap_node_config: Option<vrrb_config::BootstrapConfig>,
    pub events_tx: EventPublisher,
}

impl NetworkModule {
    pub async fn new(config: NetworkModuleConfig) -> Result<Self> {
        let dyswarm_server_config = ServerConfig { addr: config.addr };

        let dyswarm_server = dyswarm::server::Server::new(dyswarm_server_config).await?;

        let dyswarm_client_config = dyswarm::client::Config { addr: config.addr };

        let dyswarm_client = dyswarm::client::Client::new(dyswarm_client_config).await?;

        let kademlia_node = Self::setup_kademlia_node(config.clone())?;

        let events_tx = config.events_tx;

        Ok(Self {
            kademlia_node,
            events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
            dyswarm_server,
            dyswarm_client,
        })
    }

    fn setup_kademlia_node(config: NetworkModuleConfig) -> Result<KademliaNode> {
        let kademlia_node = if let Some(bootstrap_node_config) = config.bootstrap_node_config {
            let node_key_bytes = hex::decode(bootstrap_node_config.id).map_err(|err| {
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

        Ok(kademlia_node)
    }

    /// Address this module listens on
    pub fn local_addr(&self) -> SocketAddr {
        self.dyswarm_server.public_addr()
    }

    pub fn node_ref(&self) -> &KademliaNode {
        &self.kademlia_node
    }

    pub fn node_mut(&mut self) -> &mut KademliaNode {
        &mut self.kademlia_node
    }
}

#[derive(Debug)]
pub struct NetworkModuleComponentConfig {
    pub config: NodeConfig,
    // TODO: remove this attribute
    pub node_id: NodeId,
    pub events_tx: EventPublisher,
    pub node_type: NodeType,
    pub network_events_rx: EventSubscriber,
    #[deprecated(note = "use network_events_rx instead.Dyswarm removes the need for a controller")]
    pub controller_events_rx: EventSubscriber,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub udp_gossip_address_port: u16,
    pub raptorq_gossip_address_port: u16,
    //
    // TODO: figure out how to safely remove this raptor sender
    // pub raptor_sender: Sender<RaptorBroadCastedData>,
}

#[async_trait]
impl RuntimeComponent<NetworkModuleComponentConfig, SocketAddr> for NetworkModule {
    async fn setup(
        args: NetworkModuleComponentConfig,
    ) -> crate::Result<RuntimeComponentHandle<SocketAddr>> {
        let mut network_events_rx = args.network_events_rx;

        let network_module_config = NetworkModuleConfig {
            addr: args.config.udp_gossip_address,
            bootstrap_node_config: None,
            events_tx: args.events_tx,
        };

        let network_module = NetworkModule::new(network_module_config).await?;

        let network_listen_resolved_addr = network_module.local_addr();

        let mut network_module_actor = ActorImpl::new(network_module);

        let network_handle = tokio::spawn(async move {
            network_module_actor
                .start(&mut network_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        info!("Network module is operational");

        let component_handle =
            RuntimeComponentHandle::new(network_handle, network_listen_resolved_addr);

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}

#[async_trait]
impl Handler<EventMessage> for NetworkModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        String::from("NetworkModule")
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::FetchPeers(count) => {
                let key = self.kademlia_node.node_data().id.clone();
                let closest_nodes = self
                    .kademlia_node
                    .routing_table
                    .lock()
                    .map_err(|err| TheaterError::Other(err.to_string()))?
                    .get_closest_nodes(&key, count);

                for node in closest_nodes {
                    debug!("Closest Node with Key : {:?} :{:?}", key, node);
                }
            },
            Event::DHTStoreRequest(key, value) => {
                info!(
                    "Storing into DHT Store Request: {:?}:{:?}",
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
            self.label(),
            self.id(),
        );
    }
}
