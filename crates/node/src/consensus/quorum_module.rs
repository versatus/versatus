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
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{Actor, ActorId, ActorImpl, ActorState};
use vrrb_config::{BootstrapQuorumConfig, NodeConfig, QuorumMembershipConfig};
use vrrb_core::claim::{Claim, Eligibility};

use crate::{state_reader::StateReader, NodeError, RuntimeComponent, RuntimeComponentHandle};

#[derive(Debug)]
pub struct QuorumModule<S: StateReader + Send> {
    pub(crate) id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) node_config: NodeConfig,
    pub(crate) vrrbdb_read_handle: S,
    pub(crate) membership_config: Option<QuorumMembershipConfig>,
    pub(crate) bootstrap_quorum_config: Option<BootstrapQuorumConfig>,

    /// A map of all nodes known to are available in the bootstrap quorum
    pub(crate) bootstrap_quorum_available_nodes: HashMap<NodeId, (PeerData, bool)>,
}

#[derive(Debug, Clone)]
pub struct QuorumModuleConfig<S: StateReader + Send> {
    pub events_tx: EventPublisher,
    pub vrrbdb_read_handle: S,
    pub membership_config: Option<QuorumMembershipConfig>,
    pub node_config: NodeConfig,
}

impl<S: StateReader + Send + Sync> QuorumModule<S> {
    pub fn new(cfg: QuorumModuleConfig<S>) -> Self {
        let mut bootstrap_quorum_available_nodes = HashMap::new();

        if let Some(quorum_config) = cfg.node_config.bootstrap_quorum_config.clone() {
            bootstrap_quorum_available_nodes = quorum_config
                .membership_config
                .quorum_members
                .into_iter()
                .map(|member| {
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
            vrrbdb_read_handle: cfg.vrrbdb_read_handle,
            events_tx: cfg.events_tx,
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
    ) -> crate::Result<()> {
        let node_id = peer_data.node_id.clone();
        let assigned_membership = AssignedQuorumMembership {
            quorum_kind,
            node_id: node_id.clone(),
            kademlia_peer_id: peer_data.kademlia_peer_id,
            peers: peers
                .into_iter()
                .filter(|peer| peer.node_id != node_id)
                .collect::<Vec<PeerData>>(),
        };
        let event = Event::QuorumMembershipAssigmentCreated(assigned_membership).into();

        let em = EventMessage::new(Some("network-events".into()), event);

        self.events_tx.send(em).await?;

        Ok(())
    }

    pub(super) async fn assign_peer_list_to_quorums(
        &self,
        peer_list: HashMap<NodeId, (PeerData, bool)>,
    ) -> crate::Result<()> {
        let unassigned_peers = peer_list
            .into_iter()
            .filter(|(_, (peer_data, _))| peer_data.node_type == NodeType::Validator)
            .map(|(_, (peer_data, _))| peer_data)
            .collect::<Vec<PeerData>>();

        // NOTE: select 30% of nodes to be harvester nodes and make the rest farmers
        let unassigned_peers_count = unassigned_peers.len();
        let harvester_count = (unassigned_peers_count as f64 * 0.3).ceil() as usize;

        // TODO: pick nodes at random
        let mut harvester_peers = unassigned_peers
            .clone()
            .into_iter()
            .take(harvester_count)
            .collect::<Vec<PeerData>>();

        for intended_harvester in harvester_peers.iter() {
            self.assign_membership_to_quorum(
                QuorumKind::Harvester,
                intended_harvester.clone(),
                harvester_peers.clone(),
            )
            .await?;
        }

        for intended_farmer in unassigned_peers.iter() {
            self.assign_membership_to_quorum(
                QuorumKind::Farmer,
                intended_farmer.clone(),
                unassigned_peers.clone(),
            )
            .await?;
        }

        Ok(())
    }

    fn elect_quorum(
        &self,
        claims: HashMap<NodeId, Claim>,
        header: BlockHeader,
    ) -> Result<Quorum, QuorumError> {
        let last_block_height = header.block_height;
        let seed = header.next_block_seed;

        if let Ok(mut quorum) = Quorum::new(seed, last_block_height) {
            let claim_vec: Vec<Claim> = claims.values().cloned().collect();
            if let Ok(elected_quorum) = quorum.run_election(claim_vec) {
                return Ok(elected_quorum.clone());
            }
        }

        Err(QuorumError::InvalidSeedError)
    }

    fn elect_miner(
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

    fn get_winner(election_results: &mut BTreeMap<U256, Claim>) -> (U256, Claim) {
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
