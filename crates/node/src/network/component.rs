use std::{
    collections::HashMap,
    net::{AddrParseError, SocketAddr},
    ops::AddAssign,
};

use async_trait::async_trait;
use block::ConvergenceBlock;
use dyswarm::{
    client::{BroadcastArgs, BroadcastConfig},
    server::ServerConfig,
};
use events::{AssignedQuorumMembership, Event, EventMessage, EventPublisher, EventSubscriber};
use hbbft::{crypto::PublicKey as ThresholdSignaturePublicKey, sync_key_gen::Part};
use kademlia_dht::{Key, Node as KademliaNode, NodeData};
use primitives::{KademliaPeerId, NodeId, PublicKey};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use tracing::Subscriber;
use utils::payload::digest_data_to_bytes;
use vrrb_config::{BootstrapQuorumConfig, NodeConfig, QuorumMembershipConfig};
use vrrb_core::claim::Claim;

use super::NetworkEvent;
use crate::{
    network::DyswarmHandler, result::Result, NodeError, RuntimeComponent, RuntimeComponentHandle,
    DEFAULT_ERASURE_COUNT,
};

use crate::network::module::*;

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
    pub validator_public_key: PublicKey,
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
        let node_config = args.config.clone();

        let network_module_config = NetworkModuleConfig {
            node_id: args.node_id.clone(),
            node_type: args.config.node_type,
            udp_gossip_addr: args.config.udp_gossip_address,
            raptorq_gossip_addr: args.config.raptorq_gossip_address,
            kademlia_peer_id: args.config.kademlia_peer_id,
            kademlia_liveness_addr: args.config.kademlia_liveness_address,
            bootstrap_node_config: args.config.bootstrap_config,
            events_tx: args.events_tx,
            membership_config: args.membership_config,
            validator_public_key: args.validator_public_key,
            node_config,
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
