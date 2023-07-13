use std::{
    collections::{BTreeMap, HashMap},
    default,
};

use async_trait::async_trait;
use block::header::BlockHeader;
use ethereum_types::U256;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, PeerData};
use primitives::NodeId;
use quorum::{
    election::Election,
    quorum::{Quorum, QuorumError},
};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler};
use vrrb_config::{QuorumKind, QuorumMember};
use vrrb_core::claim::{Claim, Eligibility};

use crate::{NodeError, RuntimeComponent, RuntimeComponentHandle};

#[derive(Debug)]
pub struct QuorumModule {
    id: ActorId,
    status: ActorState,
    events_tx: EventPublisher,
    vrrbdb_read_handle: VrrbDbReadHandle,
    membership_config: QuorumMembershipConfig,
}

#[derive(Debug, Default)]
pub struct QuorumMembershipConfig {
    pub quorum_kind: QuorumKind,
    pub quorum_members: Vec<QuorumMember>,
}

#[derive(Debug)]
pub struct QuorumModuleConfig {
    pub events_tx: EventPublisher,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub membership_config: QuorumMembershipConfig,
}

impl QuorumModule {
    pub fn new(cfg: QuorumModuleConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ActorState::Stopped,
            vrrbdb_read_handle: cfg.vrrbdb_read_handle,
            events_tx: cfg.events_tx,
            membership_config: Default::default(),
        }
    }

    /// Replaces the current quorum membership configuration to the given one.
    pub fn reconfigure_quorum_membership(&mut self, membership_config: QuorumMembershipConfig) {
        self.membership_config = membership_config;
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

#[async_trait]
impl Handler<EventMessage> for QuorumModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("QuorumModule::{}", self.id())
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{} starting", self.label());
    }

    fn on_stop(&self) {
        info!("{} received stop signal. Stopping", self.label());
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },

            Event::PeerJoined(peer_data) => {
                // NOTE: check whether whitelisted peers joined the network to trigger a genesis
                // block generation event

                let matching_peers = self
                    .membership_config
                    .quorum_members
                    .iter()
                    .filter(|whitelisted_member| {
                        peer_data.node_id == whitelisted_member.node_id
                            && peer_data.udp_gossip_addr == whitelisted_member.udp_gossip_address
                            && peer_data.raptorq_gossip_addr
                                == whitelisted_member.raptorq_gossip_address
                    })
                    .collect::<Vec<&QuorumMember>>();

                if !matching_peers.is_empty() {
                    dbg!("whitelisted peers joined network, generating genesis block");
                }
            },

            // TODO: refactor these event handlers to properly match architecture
            // Event::QuorumElection(header) => {
            //     let claims = self.vrrbdb_read_handle.claim_store_values();
            //
            //     if let Ok(quorum) = self.elect_quorum(claims, header) {
            //         if let Err(err) = self
            //             .events_tx
            //             .send(Event::ElectedQuorum(quorum).into())
            //             .await
            //         {
            //             telemetry::error!("{}", err);
            //         }
            //     }
            // },
            // Event::MinerElection(header) => {
            //     let claims = self.vrrbdb_read_handle.claim_store_values();
            //     let mut election_results: BTreeMap<U256, Claim> =
            //         self.elect_miner(claims, header.block_seed);
            //
            //     let winner = Self::get_winner(&mut election_results);
            //
            //     if let Err(err) = self
            //         .events_tx
            //         .send(Event::ElectedMiner(winner).into())
            //         .await
            //     {
            //         telemetry::error!("{}", err);
            //     }
            // },
            _ => {},
        }
        Ok(ActorState::Running)
    }
}

#[derive(Debug)]
pub struct QuorumModuleComponentConfig {
    pub events_tx: EventPublisher,
    pub quorum_events_rx: EventSubscriber,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub membership_config: QuorumMembershipConfig,
}

#[async_trait]
impl RuntimeComponent<QuorumModuleComponentConfig, ()> for QuorumModule {
    async fn setup(args: QuorumModuleComponentConfig) -> crate::Result<RuntimeComponentHandle<()>> {
        let module = QuorumModule::new(QuorumModuleConfig {
            events_tx: args.events_tx,
            vrrbdb_read_handle: args.vrrbdb_read_handle,
            // TODO: read from config
            membership_config: args.membership_config,
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
