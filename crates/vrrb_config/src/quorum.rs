use std::{collections::BTreeMap, net::SocketAddr};

use primitives::{KademliaPeerId, NodeId, NodeType, PublicKey, QuorumKind};
use serde::{Deserialize, Serialize};

use crate::BootstrapQuorumMember;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct QuorumMember {
    pub node_id: NodeId,
    pub quorum_kind: QuorumKind,
    pub kademlia_peer_id: KademliaPeerId,
    pub node_type: NodeType,
    pub udp_gossip_address: SocketAddr,
    pub raptorq_gossip_address: SocketAddr,
    pub kademlia_liveness_address: SocketAddr,
    pub validator_public_key: PublicKey,
}

pub type QuorumMembers = BTreeMap<NodeId, QuorumMember>;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuorumMembershipConfig {
    pub quorum_kind: QuorumKind,
    pub quorum_members: BTreeMap<NodeId, QuorumMember>,
}

impl QuorumMembershipConfig {
    pub fn quorum_members(&self) -> QuorumMembers {
        self.quorum_members.clone()
    }

    pub fn quorum_kind(&self) -> QuorumKind {
        self.quorum_kind.clone()
    }
}

impl From<BootstrapQuorumMember> for QuorumMember {
    fn from(member: BootstrapQuorumMember) -> Self {
        Self {
            node_id: member.node_id,
            quorum_kind: member.quorum_kind,
            kademlia_peer_id: member.kademlia_peer_id,
            node_type: member.node_type,
            udp_gossip_address: member.udp_gossip_address,
            raptorq_gossip_address: member.raptorq_gossip_address,
            kademlia_liveness_address: member.kademlia_liveness_address,
            validator_public_key: member.validator_public_key,
        }
    }
}
