use std::{collections::HashMap, net::SocketAddr, ops::AddAssign};

use async_trait::async_trait;
use dyswarm::{
    client::{BroadcastArgs, BroadcastConfig},
    server::ServerConfig,
};
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use primitives::{KademliaPeerId, NodeId, NodeType};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use tracing::Subscriber;
use utils::payload::digest_data_to_bytes;
use vrrb_config::{BootstrapQuorumConfig, NodeConfig, QuorumMembershipConfig};
use vrrb_core::claim::Claim;

use super::NetworkEvent;
use crate::{
    network::DyswarmHandler,
    result::Result,
    NodeError,
    RuntimeComponent,
    RuntimeComponentHandle,
    DEFAULT_ERASURE_COUNT,
};

#[derive(Debug)]
pub struct NetworkModule {
    pub(crate) id: ActorId,
    pub(crate) node_id: NodeId,
    pub(crate) node_type: NodeType,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) is_bootstrap: bool,
    pub(crate) kademlia_node: KademliaNode,
    pub(crate) udp_gossip_addr: SocketAddr,
    pub(crate) raptorq_gossip_addr: SocketAddr,
    pub(crate) kademlia_liveness_addr: SocketAddr,
    pub(crate) dyswarm_server_handle: dyswarm::server::ServerHandle,
    pub(crate) dyswarm_client: dyswarm::client::Client,
    pub(crate) membership_config: Option<QuorumMembershipConfig>,
    pub(crate) bootstrap_quorum_config: Option<BootstrapQuorumConfig>,

    /// A map of all nodes known to are available in the bootstrap quorum
    pub(crate) bootstrap_quorum_available_nodes: HashMap<NodeId, bool>,
}

#[derive(Debug, Clone)]
pub struct NetworkModuleConfig {
    pub node_id: NodeId,

    pub node_type: NodeType,

    /// Address used by Dyswarm to listen for protocol events
    pub udp_gossip_addr: SocketAddr,

    /// Address used by Dyswarm to listen for protocol events via RaptorQ
    pub raptorq_gossip_addr: SocketAddr,

    /// Address used to listen for liveness pings
    pub kademlia_liveness_addr: SocketAddr,

    /// Configuration used to connect to a bootstrap node
    pub bootstrap_node_config: Option<vrrb_config::BootstrapConfig>,

    pub membership_config: Option<QuorumMembershipConfig>,

    pub bootstrap_quorum_config: Option<BootstrapQuorumConfig>,

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

        let events_tx = config.events_tx.clone();

        let handler = DyswarmHandler::new(config.node_id.clone(), events_tx.clone());

        let dyswarm_server_handle = dyswarm_server.run(handler).await?;

        let mut bootstrap_quorum_available_nodes = HashMap::new();

        if let Some(quorum_config) = config.bootstrap_quorum_config.clone() {
            bootstrap_quorum_available_nodes = quorum_config
                .membership_config
                .quorum_members
                .into_iter()
                .map(|membership| (membership.member.node_id, false))
                .collect::<HashMap<NodeId, bool>>();
        }

        let mut network_component = Self {
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            node_id: config.node_id.clone(),
            node_type: config.node_type,
            status: ActorState::Stopped,

            // NOTE: if there's bootstrap config, this node is a bootstrap node
            is_bootstrap: config.bootstrap_node_config.is_none(),
            kademlia_node,
            kademlia_liveness_addr: config.kademlia_liveness_addr,
            udp_gossip_addr: config.udp_gossip_addr,
            raptorq_gossip_addr: config.raptorq_gossip_addr,
            dyswarm_server_handle,
            dyswarm_client,
            membership_config: config.membership_config.clone(),
            bootstrap_quorum_available_nodes,
            bootstrap_quorum_config: config.bootstrap_quorum_config.clone(),
        };

        // TODO: revisit on-startup liveness checks later
        // network_component
        //     .verify_bootstrap_quorum_members_are_online(&config)
        //     .await;

        Ok(network_component)
    }

    pub async fn verify_bootstrap_quorum_members_are_online(
        &mut self,
        config: &NetworkModuleConfig,
    ) {
        if let Some(bootstrap_quorum_config) = config.bootstrap_quorum_config.clone() {
            let mut acks: usize = 0;

            let members_count = bootstrap_quorum_config
                .membership_config
                .quorum_members
                .len();

            // TODO: check if all quorum members are alive
            // miner can produce a genesis block
            for membership in bootstrap_quorum_config.membership_config.quorum_members {
                dbg!(&membership.member.kademlia_peer_id);

                let node_data = self.kademlia_node.get(&membership.member.kademlia_peer_id);

                // if let Some(_) = kademlia_node.rpc_ping(&node_data) {
                //     // NOTE: count this acknowledgement
                //     acks.add_assign(1);
                // }
            }

            if acks >= members_count {
                let event = Event::GenesisQuorumMembersAvailable;
                self.events_tx.send(event.into());
            }
        }
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
            )?
        } else {
            // NOTE: become a bootstrap node if no bootstrap info is provided
            info!("Becoming a bootstrap node");

            KademliaNode::new(config.kademlia_liveness_addr, config.udp_gossip_addr, None)?
        };

        Ok(kademlia_node)
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    pub fn is_bootstrap(&self) -> bool {
        self.is_bootstrap
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

    async fn broadcast_join_intent(&mut self) -> Result<()> {
        let msg = dyswarm::types::Message::new(NetworkEvent::PeerJoined {
            node_id: self.node_id.clone(),
            node_type: self.node_type(),
            kademlia_peer_id: self.kademlia_peer_id(),
            udp_gossip_addr: self.udp_gossip_addr(),
            raptorq_gossip_addr: self.raptorq_gossip_addr(),
            kademlia_liveness_addr: self.kademlia_liveness_addr(),
        });

        let nid = self.kademlia_node.node_data().id;
        let rt = self.kademlia_node.get_routing_table();
        let closest_nodes = rt.get_closest_nodes(&nid, 7);

        let closest_nodes_udp_addrs = closest_nodes
            .clone()
            .into_iter()
            .map(|n| n.udp_gossip_addr)
            .collect();

        self.dyswarm_client
            .add_peers(closest_nodes_udp_addrs)
            .await?;

        let args = BroadcastArgs {
            config: BroadcastConfig { unreliable: false },
            message: msg.clone(),
            erasure_count: DEFAULT_ERASURE_COUNT,
        };

        if let Err(err) = self.dyswarm_client.broadcast(args).await {
            telemetry::warn!("Failed to broadcast join intent: {err}");
        }

        Ok(())
    }

    pub(crate) async fn broadcast_claim(&mut self, claim: Claim) -> Result<()> {
        let closest_nodes = self
            .node_ref()
            .get_routing_table()
            .get_closest_nodes(&self.node_ref().node_data().id, 8);

        let socket_address = closest_nodes
            .iter()
            .map(|node| node.udp_gossip_addr)
            .collect();

        self.dyswarm_client.add_peers(socket_address).await?;

        let node_id = self.node_id.clone();

        let message = dyswarm::types::Message::new(NetworkEvent::ClaimCreated { node_id, claim });

        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct NetworkModuleComponentConfig {
    pub config: NodeConfig,

    // TODO: remove this attribute
    pub node_id: NodeId,
    pub events_tx: EventPublisher,
    pub network_events_rx: EventSubscriber,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub membership_config: Option<QuorumMembershipConfig>,
    pub bootstrap_quorum_config: Option<BootstrapQuorumConfig>,
}

#[derive(Debug, Clone)]
pub struct NetworkModuleComponentResolvedData {
    pub kademlia_peer_id: KademliaPeerId,
    pub resolved_kademlia_liveness_address: SocketAddr,
    pub resolved_udp_gossip_address: SocketAddr,
    pub resolved_raptorq_gossip_address: SocketAddr,
}

#[async_trait]
impl RuntimeComponent<NetworkModuleComponentConfig, NetworkModuleComponentResolvedData>
    for NetworkModule
{
    async fn setup(
        args: NetworkModuleComponentConfig,
    ) -> crate::Result<RuntimeComponentHandle<NetworkModuleComponentResolvedData>> {
        let mut network_events_rx = args.network_events_rx;

        let network_module_config = NetworkModuleConfig {
            node_id: args.node_id.clone(),
            node_type: args.config.node_type,
            udp_gossip_addr: args.config.udp_gossip_address,
            raptorq_gossip_addr: args.config.raptorq_gossip_address,
            kademlia_liveness_addr: args.config.kademlia_liveness_address,
            bootstrap_node_config: args.config.bootstrap_config,
            events_tx: args.events_tx,
            membership_config: args.membership_config,
            bootstrap_quorum_config: args.bootstrap_quorum_config,
        };

        let mut network_module = NetworkModule::new(network_module_config).await?;
        let label = network_module.label();

        let resolved_udp_gossip_address = network_module.udp_gossip_addr();
        let kademlia_dht_resolved_id = network_module.kademlia_peer_id();
        let resolved_kademlia_liveness_address = network_module.kademlia_liveness_addr();
        let resolved_raptorq_gossip_address = network_module.raptorq_gossip_addr();

        let is_not_bootstrap = !network_module.is_bootstrap();

        if is_not_bootstrap {
            network_module.broadcast_join_intent().await?;
        }

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
            RuntimeComponentHandle::new(network_handle, network_component_resolved_data, label);

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}
