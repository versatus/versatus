use std::net::SocketAddr;

use async_trait::async_trait;
use dyswarm::server::ServerConfig;
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use patriecia::Database;
use primitives::{KademliaPeerId, NodeId, NodeType};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler};
use tracing::debug;
use utils::payload::digest_data_to_bytes;
use vrrb_config::NodeConfig;

use super::NetworkEvent;
use crate::{
    components::network::DyswarmHandler, result::Result, NodeError, RuntimeComponent,
    RuntimeComponentHandle,
};

#[derive(Debug)]
pub struct NetworkModule {
    id: ActorId,
    label: ActorLabel,
    node_id: NodeId,
    status: ActorState,
    events_tx: EventPublisher,
    kademlia_node: KademliaNode,
    udp_gossip_addr: SocketAddr,
    raptorq_gossip_addr: SocketAddr,
    kademlia_liveness_addr: SocketAddr,
    dyswarm_server_handle: dyswarm::server::ServerHandle,
    dyswarm_client: dyswarm::client::Client,
}

#[derive(Debug, Clone)]
pub struct NetworkModuleConfig {
    pub node_id: NodeId,

    /// Address used by Dyswarm to listen for protocol events
    pub udp_gossip_addr: SocketAddr,

    /// Address used by Dyswarm to listen for protocol events via RaptorQ
    pub raptorq_gossip_addr: SocketAddr,

    /// Address used to listen for liveness pings
    pub kademlia_liveness_addr: SocketAddr,

    /// Configuration used to connect to a bootstrap node
    pub bootstrap_node_config: Option<vrrb_config::BootstrapConfig>,
    pub events_tx: EventPublisher,
}

impl NetworkModule {
    pub async fn new(config: NetworkModuleConfig) -> Result<Self> {
        let mut config = config.clone();

        let dyswarm_server_config = ServerConfig {
            addr: config.udp_gossip_addr,
        };

        let dyswarm_server = dyswarm::server::Server::new(dyswarm_server_config).await?;

        let resolved_addr = dyswarm_server.public_addr();
        config.udp_gossip_addr = resolved_addr;

        let dyswarm_client_config = dyswarm::client::Config {
            addr: config.udp_gossip_addr,
        };

        let dyswarm_client = dyswarm::client::Client::new(dyswarm_client_config).await?;

        let kademlia_node = Self::setup_kademlia_node(config.clone())?;
        config.kademlia_liveness_addr = kademlia_node.node_data().addr;

        let events_tx = config.events_tx;

        let handler = DyswarmHandler::new(config.node_id.clone(), events_tx.clone());

        let dyswarm_server_handle = dyswarm_server.run(handler).await?;

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            node_id: config.node_id,
            label: String::from("State"),
            status: ActorState::Stopped,
            kademlia_node,
            kademlia_liveness_addr: config.kademlia_liveness_addr,
            udp_gossip_addr: config.udp_gossip_addr,
            raptorq_gossip_addr: config.raptorq_gossip_addr,
            dyswarm_server_handle,
            dyswarm_client,
        })
    }

    fn setup_kademlia_node(config: NetworkModuleConfig) -> Result<KademliaNode> {
        // TODO: inspect that nodes are being created with the correct config when a
        // bootstrap is provided
        // TODO: provide safeguards to prevent nodes calling themselves bootstraps when
        // there's another one already running. Consider this a critical error
        // and a protocol concern
        //
        let kademlia_node = if let Some(bootstrap_node_config) = config.bootstrap_node_config {
            // NOTE: turns a node's id into a 32 byte array
            let node_key_bytes = digest_data_to_bytes(&bootstrap_node_config.id);

            let kademlia_key = Key::try_from(node_key_bytes).map_err(|err| {
                NodeError::Other(format!("Node key should have a 32 byte length: {err}"))
            })?;

            // TODO: figure out why kademlia_dht needs the ip, port and then the whole
            // address separately
            // NOTE: this snippet turns the bootstrap node config into a NodeData struct
            // that kademlia_dht understands
            let bootstrap_node_data = NodeData::new(
                kademlia_key,
                bootstrap_node_config.kademlia_liveness_addr,
                bootstrap_node_config.udp_gossip_addr,
            );

            KademliaNode::new(
                config.kademlia_liveness_addr,
                config.udp_gossip_addr,
                Some(bootstrap_node_data),
            )
        } else {
            // NOTE: become a bootstrap node if no bootstrap info is provided
            info!("Becoming a bootstrap node");
            KademliaNode::new(config.kademlia_liveness_addr, config.udp_gossip_addr, None)
        }?;

        Ok(kademlia_node)
    }

    /// Address this module listens on for network events via UDP
    // NOTE: currently assume UDP is the primary means of communication however this
    // may not be entirely accurate in the near future.
    pub fn local_addr(&self) -> SocketAddr {
        self.udp_gossip_addr()
    }

    /// Address this module listens on for network events via UDP
    pub fn udp_gossip_addr(&self) -> SocketAddr {
        self.udp_gossip_addr
    }

    pub fn dp_gossip_addr(&self) -> SocketAddr {
        self.udp_gossip_addr
    }

    /// Address this module listens on for network events via RaptorQ
    pub fn raptorq_gossip_addr(&self) -> SocketAddr {
        self.raptorq_gossip_addr
    }

    /// ID used by Kademlia DHT to identify this node
    pub fn kademlia_peer_id(&self) -> KademliaPeerId {
        self.kademlia_node.node_data().id
    }

    /// Address this module listens on for liveness pings
    pub fn kademlia_liveness_addr(&self) -> SocketAddr {
        self.kademlia_node.node_data().addr
    }

    pub fn node_ref(&self) -> &KademliaNode {
        &self.kademlia_node
    }

    pub fn node_mut(&mut self) -> &mut KademliaNode {
        &mut self.kademlia_node
    }
}

#[derive(Debug)]
pub struct NetworkModuleComponentConfig<D: Database> {
    pub config: NodeConfig,
    // TODO: remove this attribute
    pub node_id: NodeId,
    pub events_tx: EventPublisher,
    pub node_type: NodeType,
    pub network_events_rx: EventSubscriber,
    pub vrrbdb_read_handle: VrrbDbReadHandle<D>,
    //
    // TODO: figure out how to safely remove this raptor sender
    // pub raptor_sender: Sender<RaptorBroadCastedData>,
}

#[derive(Debug, Clone)]
pub struct NetworkModuleComponentResolvedData {
    pub kademlia_peer_id: KademliaPeerId,
    pub resolved_kademlia_liveness_address: SocketAddr,
    pub resolved_udp_gossip_address: SocketAddr,
    pub resolved_raptorq_gossip_address: SocketAddr,
}

#[async_trait]
impl<D: Database + 'static>
    RuntimeComponent<NetworkModuleComponentConfig<D>, NetworkModuleComponentResolvedData>
    for NetworkModule
{
    async fn setup(
        args: NetworkModuleComponentConfig<D>,
    ) -> crate::Result<RuntimeComponentHandle<NetworkModuleComponentResolvedData>> {
        let mut network_events_rx = args.network_events_rx;

        let network_module_config = NetworkModuleConfig {
            node_id: args.node_id,
            udp_gossip_addr: args.config.udp_gossip_address,
            raptorq_gossip_addr: args.config.raptorq_gossip_address,
            kademlia_liveness_addr: args.config.kademlia_liveness_address,
            bootstrap_node_config: args.config.bootstrap_config,
            events_tx: args.events_tx,
        };

        let network_module = NetworkModule::new(network_module_config).await?;

        let resolved_udp_gossip_address = network_module.udp_gossip_addr();
        let kademlia_dht_resolved_id = network_module.kademlia_peer_id();
        let resolved_kademlia_liveness_address = network_module.kademlia_liveness_addr();
        let resolved_raptorq_gossip_address = network_module.raptorq_gossip_addr();

        let mut network_module_actor = ActorImpl::new(network_module);

        let network_handle = tokio::spawn(async move {
            network_module_actor
                .start(&mut network_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        info!("Network module is operational");

        let network_component_resolved_data = NetworkModuleComponentResolvedData {
            kademlia_peer_id: kademlia_dht_resolved_id,
            resolved_kademlia_liveness_address,
            resolved_udp_gossip_address,
            resolved_raptorq_gossip_address,
        };

        let component_handle =
            RuntimeComponentHandle::new(network_handle, network_component_resolved_data);

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        // self.dyswarm_server_handle.stop().await;
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
                let key = self.node_ref().node_data().id.clone();
                let closest_nodes = self
                    .node_ref()
                    .get_routing_table()
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

            // TODO: remove all that experimental code below and replace it with the appropriate
            // behavior
            Event::Ping(node_id) => {
                let timestamp = chrono::Utc::now().timestamp();

                let msg = dyswarm::types::Message {
                    id: dyswarm::types::MessageId::new_v4(),
                    timestamp,
                    data: NetworkEvent::Ping(node_id),
                };

                let nid = self.kademlia_node.node_data().id;
                let rtt = self.kademlia_node.get_routing_table();
                let closest_nodes = rtt.get_closest_nodes(&nid, 7);

                // TODO: store additional data within kademlia nodes. later look at multiplexing
                // data+kademlia over a single port
                for node_data in closest_nodes {
                    self.dyswarm_client
                        .send_data_via_quic(msg.clone(), node_data.udp_gossip_addr)
                        .await
                        .unwrap();
                }
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
