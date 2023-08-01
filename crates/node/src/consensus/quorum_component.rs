use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use block::header::BlockHeader;
use ethereum_types::U256;
use events::{
    AssignedQuorumMembership,
    Event,
    EventMessage,
    EventPublisher,
    EventSubscriber,
    PeerData,
};
use primitives::{NodeId, NodeType};
use quorum::{
    election::Election,
    quorum::{Quorum, QuorumError},
};
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{Actor, ActorId, ActorImpl, ActorState};
use vrrb_config::{
    BootstrapQuorumConfig,
    NodeConfig,
    QuorumKind,
    QuorumMembership,
    QuorumMembershipConfig,
};
use vrrb_core::claim::{Claim, Eligibility};

use crate::{NodeError, RuntimeComponent, RuntimeComponentHandle};

#[derive(Debug)]
pub struct QuorumModule {
    pub(crate) id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) node_config: NodeConfig,
    pub(crate) vrrbdb_read_handle: VrrbDbReadHandle,
    pub(crate) membership_config: Option<QuorumMembershipConfig>,
    pub(crate) bootstrap_quorum_config: Option<BootstrapQuorumConfig>,

    /// A map of all nodes known to are available in the bootstrap quorum
    pub(crate) bootstrap_quorum_available_nodes: HashMap<NodeId, (PeerData, bool)>,
}

#[derive(Debug, Clone)]
pub struct QuorumModuleConfig {
    pub events_tx: EventPublisher,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
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
                .map(|membership| {
                    let peer = PeerData {
                        node_id: membership.member.node_id,
                        node_type: membership.member.node_type,
                        kademlia_peer_id: membership.member.kademlia_peer_id,
                        udp_gossip_addr: membership.member.udp_gossip_address,
                        raptorq_gossip_addr: membership.member.raptorq_gossip_address,
                        kademlia_liveness_addr: membership.member.kademlia_liveness_address,
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

    async fn assign_membership_to_quorum(&self, peer_data: PeerData) -> crate::Result<()> {
        let assigned_membership = AssignedQuorumMembership {
            node_id: peer_data.node_id,
            kademlia_peer_id: peer_data.kademlia_peer_id,
        };

        self.events_tx
            .send(Event::QuorumMembershipAssigned(assigned_membership).into())
            .await?;

        Ok(())
    }

    pub(super) async fn assign_peer_list_to_quorums(
        &self,
        peer_list: HashMap<NodeId, (PeerData, bool)>,
    ) -> crate::Result<()> {
        let unassigned_peers = peer_list
            .into_iter()
            .map(|(_, (peer_data, _))| peer_data)
            .collect::<Vec<PeerData>>();

        dbg!(unassigned_peers);

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

#[derive(Debug)]
pub struct QuorumModuleComponentConfig {
    pub events_tx: EventPublisher,
    pub quorum_events_rx: EventSubscriber,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub membership_config: QuorumMembershipConfig,
    pub node_config: NodeConfig,
}

#[async_trait]
impl RuntimeComponent<QuorumModuleComponentConfig, ()> for QuorumModule {
    async fn setup(args: QuorumModuleComponentConfig) -> crate::Result<RuntimeComponentHandle<()>> {
        let module = QuorumModule::new(QuorumModuleConfig {
            events_tx: args.events_tx,
            vrrbdb_read_handle: args.vrrbdb_read_handle,
            membership_config: Some(args.membership_config),
            node_config: args.node_config,
        });

        let mut quorum_events_rx = args.quorum_events_rx;

        let mut quorum_module_actor = ActorImpl::new(module);
        let label = quorum_module_actor.label();
        let quorum_handle = tokio::spawn(async move {
            quorum_module_actor
                .start(&mut quorum_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        let component_handle = RuntimeComponentHandle::new(quorum_handle, (), label);

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}
