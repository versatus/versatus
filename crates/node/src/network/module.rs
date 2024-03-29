use std::net::SocketAddr;

use block::{Block, Certificate, ConvergenceBlock};
use dyswarm::{
    client::{BroadcastArgs, BroadcastConfig},
    server::ServerConfig,
};
use events::{AssignedQuorumMembership, EventPublisher, Vote};
use hbbft::sync_key_gen::{Ack, Part};
use kademlia_dht::{Node as KademliaNode, NodeData};
use primitives::{ConvergencePartialSig, KademliaPeerId, NodeId, NodeType, PublicKey};
use telemetry::info;
use theater::{ActorId, ActorState};
use vrrb_config::{NodeConfig, QuorumMembershipConfig};
use vrrb_core::claim::Claim;

use super::NetworkEvent;
use crate::{network::DyswarmHandler, result::Result, NodeError, DEFAULT_ERASURE_COUNT};

// TODO: change these magic numbers when retrieving the closest peers to a dynamically sized
// network members count such that broadcast can happen across the whole network

#[derive(Debug)]
pub struct NetworkModule {
    pub(crate) id: ActorId,
    pub(crate) _node_config: NodeConfig,
    pub(crate) node_id: NodeId,
    pub(crate) node_type: NodeType,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) is_bootstrap: bool,
    pub(crate) kademlia_node: KademliaNode,
    pub(crate) udp_gossip_addr: SocketAddr,
    pub(crate) raptorq_gossip_addr: SocketAddr,
    pub(crate) _kademlia_liveness_addr: SocketAddr,
    pub(crate) _dyswarm_server_handle: dyswarm::server::ServerHandle,
    pub(crate) dyswarm_client: dyswarm::client::Client,
    pub(crate) _membership_config: Option<QuorumMembershipConfig>,
    pub(crate) validator_public_key: PublicKey,
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

    pub kademlia_peer_id: Option<KademliaPeerId>,

    /// Configuration for bootstrap nodes
    pub bootstrap_node_config: Option<vrrb_config::BootstrapConfig>,

    pub bootstrap_peer_data: Option<vrrb_config::BootstrapPeerData>,

    pub membership_config: Option<QuorumMembershipConfig>,

    pub events_tx: EventPublisher,

    pub validator_public_key: PublicKey,

    pub node_config: NodeConfig,
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

        let network_component = Self {
            id: uuid::Uuid::new_v4().to_string(),
            events_tx,
            node_id: config.node_id.clone(),
            node_type: config.node_type,
            status: ActorState::Stopped,
            _node_config: config.node_config.clone(),

            // NOTE: if there's bootstrap config, this node is a bootstrap node
            is_bootstrap: config.node_config.is_bootstrap(),
            kademlia_node,
            _kademlia_liveness_addr: config.kademlia_liveness_addr,
            udp_gossip_addr: config.udp_gossip_addr,
            raptorq_gossip_addr: config.raptorq_gossip_addr,
            _dyswarm_server_handle: dyswarm_server_handle,
            dyswarm_client,
            _membership_config: config.membership_config.clone(),
            validator_public_key: config.validator_public_key,
        };

        Ok(network_component)
    }

    fn setup_kademlia_node(config: NetworkModuleConfig) -> Result<KademliaNode> {
        // TODO: inspect that nodes are being created with the correct config when a
        // bootstrap is provided
        //
        // TODO: provide safeguards to prevent nodes calling themselves bootstraps when
        // there's another one already running. Consider this a critical error
        // and a protocol concern

        // NOTE: should force the node to crash if the CLI didn't fed it a kademlia id on startup
        let kademlia_key = config.node_config.kademlia_peer_id.ok_or(NodeError::Other(
            "Kademlia ID not present within NodeConfig".into(),
        ))?;

        let kademlia_node = if let Some(bootstrap_peer_data) = config.bootstrap_peer_data {
            // TODO: figure out why kademlia_dht needs the ip, port and then the whole
            // address separately
            //
            // NOTE: this snippet turns the bootstrap node config into a NodeData struct
            // that kademlia_dht understands
            let bootstrap_node_data = NodeData::new(
                kademlia_key,
                config.node_id.clone(),
                bootstrap_peer_data.kademlia_liveness_addr,
                bootstrap_peer_data.udp_gossip_addr,
            );

            KademliaNode::new(
                Some(kademlia_key),
                config.node_id.clone(),
                config.kademlia_liveness_addr,
                config.udp_gossip_addr,
                Some(bootstrap_node_data),
            )?
        } else {
            // NOTE: become a bootstrap node if no bootstrap info is provided
            info!("Becoming a bootstrap node");

            KademliaNode::new(
                Some(kademlia_key),
                config.node_id.clone(),
                config.kademlia_liveness_addr,
                config.udp_gossip_addr,
                None,
            )?
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

    pub fn validator_public_key(&self) -> PublicKey {
        self.validator_public_key
    }

    pub async fn broadcast_join_intent(&mut self) -> Result<()> {
        let msg = dyswarm::types::Message::new(NetworkEvent::PeerJoined {
            node_id: self.node_id.clone(),
            node_type: self.node_type(),
            kademlia_peer_id: self.kademlia_peer_id(),
            udp_gossip_addr: self.udp_gossip_addr(),
            raptorq_gossip_addr: self.raptorq_gossip_addr(),
            kademlia_liveness_addr: self.kademlia_liveness_addr(),
            validator_public_key: self.validator_public_key(),
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

    pub(crate) async fn notify_quorum_membership_assignments(
        &mut self,
        assignments: Vec<AssignedQuorumMembership>,
    ) -> Result<()> {
        let closest_nodes = self
            .node_ref()
            .get_routing_table()
            // TODO: fix this hardcoded peer count
            .get_closest_nodes(&self.node_ref().node_data().id, 8);

        let socket_address = closest_nodes
            .iter()
            .map(|node| node.udp_gossip_addr)
            .collect();

        self.dyswarm_client.add_peers(socket_address).await?;

        let message = dyswarm::types::Message::new(
            NetworkEvent::QuorumMembershipAssigmentsCreated(assignments),
        );

        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

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

    pub async fn broadcast_part_commitment(&mut self, node_id: NodeId, part: Part) -> Result<()> {
        let closest_nodes = self
            .node_ref()
            .get_routing_table()
            .get_closest_nodes(&self.node_ref().node_data().id, 8);

        let socket_addresses = closest_nodes
            .iter()
            .map(|node| node.udp_gossip_addr)
            .collect();

        self.dyswarm_client.add_peers(socket_addresses).await?;

        let message =
            dyswarm::types::Message::new(NetworkEvent::PartCommitmentCreated(node_id, part));

        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

        Ok(())
    }

    pub async fn broadcast_part_commitment_acknowledgement(
        &mut self,
        node_id: NodeId,
        sender_id: NodeId,
        ack: Ack,
    ) -> Result<()> {
        let closest_nodes = self
            .node_ref()
            .get_routing_table()
            .get_closest_nodes(&self.node_ref().node_data().id, 8);

        let found_peer = closest_nodes
            .iter()
            .find(|node| node.node_id == node_id.clone())
            .ok_or(NodeError::Other(
                "Could not find peer in routing table".to_string(),
            ))?;

        let addr = found_peer.udp_gossip_addr;

        let message = dyswarm::types::Message::new(NetworkEvent::PartCommitmentAcknowledged {
            node_id,
            sender_id,
            ack,
        });

        self.dyswarm_client
            .send_data_via_quic(message, addr)
            .await?;

        Ok(())
    }

    pub async fn broadcast_certified_convergence_block(
        &mut self,
        block: ConvergenceBlock,
    ) -> Result<()> {
        let message = dyswarm::types::Message::new(NetworkEvent::ConvergenceBlockCertified(block));

        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

        Ok(())
    }

    pub async fn broadcast_convergence_block_partial_signature(
        &mut self,
        sig: ConvergencePartialSig,
    ) -> Result<()> {
        let message =
            dyswarm::types::Message::new(NetworkEvent::ConvergenceBlockPartialSignComplete(sig));

        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

        Ok(())
    }

    pub async fn broadcast_certificate(&mut self, cert: Certificate) -> Result<()> {
        let message = dyswarm::types::Message::new(NetworkEvent::BroadcastCertificate(cert));

        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

        Ok(())
    }

    pub async fn broadcast_transaction_vote(&mut self, vote: Vote) -> Result<()> {
        telemetry::info!("Broadcasting transaction vote to network");
        let message =
            dyswarm::types::Message::new(NetworkEvent::BroadcastTransactionVote(Box::new(vote)));
        self.dyswarm_client
            .broadcast(BroadcastArgs {
                config: Default::default(),
                message,
                erasure_count: 0,
            })
            .await?;

        Ok(())
    }

    pub(crate) async fn broadcast_block(&mut self, block: Block) -> Result<()> {
        let closest_nodes = self
            .node_ref()
            .get_routing_table()
            .get_closest_nodes(&self.node_ref().node_data().id, 8);

        let socket_address = closest_nodes
            .iter()
            .map(|node| node.udp_gossip_addr)
            .collect();

        self.dyswarm_client.add_peers(socket_address).await?;

        let message = dyswarm::types::Message::new(NetworkEvent::BlockCreated(block));

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
