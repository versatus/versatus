use std::net::SocketAddr;

use primitives::{KademliaPeerId, NodeId, NodeType, QuorumKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuorumMember {
    pub node_id: NodeId,
    pub kademlia_peer_id: KademliaPeerId,
    pub node_type: NodeType,
    pub udp_gossip_address: SocketAddr,
    pub raptorq_gossip_address: SocketAddr,
    pub kademlia_liveness_address: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuorumMembership {
    pub member: QuorumMember,
    pub quorum_kind: QuorumKind,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuorumMembershipConfig {
    pub quorum_members: Vec<QuorumMembership>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapQuorumConfig {
    pub membership_config: QuorumMembershipConfig,
    pub genesis_transaction_threshold: u64,
}

impl BootstrapQuorumConfig {
    pub fn membership_config(&self) -> QuorumMembershipConfig {
        self.membership_config.clone()
    }

    pub fn membership_config_ref(&self) -> &QuorumMembershipConfig {
        &self.membership_config
    }
}

impl QuorumMembershipConfig {
    pub fn members_by_quorum_kind(&self, kind: QuorumKind) -> Vec<QuorumMember> {
        self.quorum_members
            .iter()
            .filter(|m| m.quorum_kind == kind)
            .map(|m| m.member.clone())
            .collect()
    }
}
