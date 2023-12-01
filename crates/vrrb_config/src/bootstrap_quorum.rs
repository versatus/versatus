use primitives::{KademliaPeerId, NodeId, NodeType, PublicKey, QuorumKind};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapQuorumMember {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub quorum_kind: QuorumKind,
    pub kademlia_peer_id: KademliaPeerId,
    pub udp_gossip_address: SocketAddr,
    pub raptorq_gossip_address: SocketAddr,
    pub kademlia_liveness_address: SocketAddr,
    pub validator_public_key: PublicKey,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapQuorumConfig {
    pub quorum_members: BTreeMap<NodeId, BootstrapQuorumMember>,
}

impl BootstrapQuorumConfig {
    pub fn insert(&mut self, node_id: NodeId, member: BootstrapQuorumMember) {
        self.quorum_members.insert(node_id, member);
    }

    pub fn get_member(&self, node_id: &NodeId) -> Option<&BootstrapQuorumMember> {
        self.quorum_members.get(node_id)
    }

    pub fn get_member_mut(&mut self, node_id: &NodeId) -> Option<&mut BootstrapQuorumMember> {
        self.quorum_members.get_mut(node_id)
    }

    pub fn get_harvesters(&self) -> Vec<&BootstrapQuorumMember> {
        self.quorum_members
            .iter()
            .filter_map(|(_, member)| {
                if member.quorum_kind == QuorumKind::Harvester {
                    Some(member)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_farmers(&self) -> Vec<&BootstrapQuorumMember> {
        self.quorum_members
            .iter()
            .filter_map(|(_, member)| {
                if member.quorum_kind == QuorumKind::Farmer {
                    Some(member)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_miners(&self) -> Vec<&BootstrapQuorumMember> {
        self.quorum_members
            .iter()
            .filter_map(|(_, member)| {
                if member.node_type == NodeType::Miner {
                    Some(member)
                } else {
                    None
                }
            })
            .collect()
    }
}
