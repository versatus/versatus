use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use block::header::BlockHeader;
use ethereum_types::U256;
use events::{
    AssignedQuorumMembership, Event, EventMessage, EventPublisher, EventSubscriber, PeerData,
};
use primitives::{NodeId, NodeType, QuorumKind};
use quorum::{
    election::Election,
    quorum::{Quorum, QuorumError},
};
use theater::{Actor, ActorId, ActorImpl, ActorState};
use vrrb_config::{BootstrapQuorumConfig, NodeConfig, QuorumMembershipConfig};
use vrrb_core::claim::{Claim, Eligibility};

#[derive(Debug, Clone)]
pub struct QuorumModule {
    pub(crate) id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) node_config: NodeConfig,
    pub(crate) membership_config: Option<QuorumMembershipConfig>,
    pub(crate) bootstrap_quorum_config: Option<BootstrapQuorumConfig>,

    /// A map of all nodes known to are available in the bootstrap quorum
    pub(crate) bootstrap_quorum_available_nodes: HashMap<NodeId, (PeerData, bool)>,
}

#[derive(Debug, Clone)]
pub struct QuorumModuleConfig {
    pub membership_config: Option<QuorumMembershipConfig>,
    pub node_config: NodeConfig,
}

impl QuorumModule {
    pub fn new(cfg: QuorumModuleConfig) -> Self {
        let mut bootstrap_quorum_available_nodes = HashMap::new();

        if let Some(quorum_config) = cfg.node_config.bootstrap_quorum_config.clone() {
            bootstrap_quorum_available_nodes = quorum_config
                .membership_config
                .quorum_members
                .into_iter()
                .map(|(_, member)| {
                    let peer = PeerData {
                        node_id: member.node_id,
                        node_type: member.node_type,
                        kademlia_peer_id: member.kademlia_peer_id,
                        udp_gossip_addr: member.udp_gossip_address,
                        raptorq_gossip_addr: member.raptorq_gossip_address,
                        kademlia_liveness_addr: member.kademlia_liveness_address,
                        validator_public_key: member.validator_public_key,
                    };

                    (peer.node_id.clone(), (peer, false))
                })
                .collect::<HashMap<NodeId, (PeerData, bool)>>();
        }

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ActorState::Stopped,
            membership_config: None,
            node_config: cfg.node_config.clone(),
            bootstrap_quorum_config: cfg.node_config.bootstrap_quorum_config.clone(),
            bootstrap_quorum_available_nodes,
        }
    }

    /// Replaces the current quorum membership configuration to the given one.
    pub fn reconfigure_quorum_membership(&mut self, membership_config: QuorumMembershipConfig) {
        self.membership_config = Some(membership_config);
    }

    async fn assign_membership_to_quorum(
        &self,
        quorum_kind: QuorumKind,
        peer_data: PeerData,
        peers: Vec<PeerData>,
    ) -> crate::Result<AssignedQuorumMembership> {
        let node_id = peer_data.node_id.clone();
        let assigned_membership = AssignedQuorumMembership {
            quorum_kind,
            node_id: node_id.clone(),
            pub_key: peer_data.validator_public_key,
            kademlia_peer_id: peer_data.kademlia_peer_id,
            peers: peers
                .into_iter()
                .filter(|peer| peer.node_id != node_id)
                .collect::<Vec<PeerData>>(),
        };

        Ok(assigned_membership)
    }

    // TODO: refactor to return a list of assigned quorum members instead so the handler can emit
    // the event
    pub(super) async fn assign_peer_list_to_quorums(
        &self,
        peer_list: HashMap<NodeId, (PeerData, bool)>,
    ) -> crate::Result<HashMap<NodeId, AssignedQuorumMembership>> {
        let unassigned_miner_peers = peer_list
            .iter()
            .filter(|(_, (peer_data, _))| peer_data.node_type == NodeType::Miner)
            .map(|(_, (peer_data, _))| peer_data)
            .cloned()
            .collect::<Vec<PeerData>>();

        let unassigned_peers = peer_list
            .iter()
            .filter(|(_, (peer_data, _))| peer_data.node_type == NodeType::Validator)
            .map(|(_, (peer_data, _))| peer_data)
            .cloned()
            .collect::<Vec<PeerData>>();

        // NOTE: select 30% of nodes to be harvester nodes and make the rest farmers
        let unassigned_peers_count = unassigned_peers.len();
        let harvester_count = (unassigned_peers_count as f64 * 0.3).ceil() as usize;

        // TODO: pick nodes at random
        let harvester_peers = unassigned_peers
            .clone()
            .into_iter()
            .take(harvester_count)
            .collect::<Vec<PeerData>>();

        let mut quorum_assignments = HashMap::new();

        for intended_harvester in harvester_peers.iter() {
            let id = intended_harvester.node_id.clone();
            let assignment = self
                .assign_membership_to_quorum(
                    QuorumKind::Harvester,
                    intended_harvester.clone(),
                    harvester_peers.clone(),
                )
                .await?;

            quorum_assignments.insert(id, assignment);
        }

        for intended_farmer in unassigned_peers.iter().skip(harvester_count) {
            let id = intended_farmer.node_id.clone();
            let assignment = self
                .assign_membership_to_quorum(
                    QuorumKind::Farmer,
                    intended_farmer.clone(),
                    unassigned_peers.clone(),
                )
                .await?;

            quorum_assignments.insert(id, assignment);
        }

        for intended_miner in unassigned_miner_peers.iter() {
            let id = intended_miner.node_id.clone();
            let assignment = self
                .assign_membership_to_quorum(
                    QuorumKind::Miner,
                    intended_miner.clone(),
                    unassigned_miner_peers.clone(),
                )
                .await?;

            quorum_assignments.insert(id, assignment);
        }

        Ok(quorum_assignments)
    }

    pub fn elect_quorums(
        &self,
        claims: HashMap<NodeId, Claim>,
        header: BlockHeader,
    ) -> Result<Vec<Quorum>, QuorumError> {
        let last_block_height = header.block_height;
        let seed = header.next_block_seed;

        if let Ok(mut quorum) = Quorum::new(seed, last_block_height, None) {
            let claim_vec: Vec<Claim> = claims.values().cloned().collect();
            if let Ok(elected_quorum) = quorum.run_election(claim_vec) {
                return Ok(elected_quorum.clone());
            }
        }

        Err(QuorumError::InvalidSeedError)
    }

    pub(crate) fn elect_miner(
        &self,
        claims: HashMap<NodeId, Claim>,
        block_seed: u64,
    ) -> BTreeMap<U256, Claim> {
        claims
            .iter()
            .filter(|(_, claim)| claim.eligibility == Eligibility::Miner)
            .map(|(_nodeid, claim)| Self::single_miner_results(claim, block_seed))
            .collect()
    }

    fn single_miner_results(claim: &Claim, block_seed: u64) -> (U256, Claim) {
        (claim.get_election_result(block_seed), claim.clone())
    }

    pub(crate) fn get_winner(&self, election_results: &mut BTreeMap<U256, Claim>) -> (U256, Claim) {
        let mut iter = election_results.iter();
        let first: (U256, Claim);
        loop {
            if let Some((pointer_sum, claim)) = iter.next() {
                first = (*pointer_sum, claim.clone());
                break;
            }
        }

        first
    }
}
