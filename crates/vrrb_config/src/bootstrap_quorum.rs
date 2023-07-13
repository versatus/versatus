use std::net::SocketAddr;

use primitives::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum QuorumKind {
    #[default]
    Harvester,
    Farmer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumMember {
    pub node_id: NodeId,
    pub node_type: QuorumKind,
    pub udp_gossip_address: SocketAddr,
    pub raptorq_gossip_address: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapQuorumConfig {
    pub quorum_members: Vec<QuorumMember>,
    pub quorum_kind: QuorumKind,
}
