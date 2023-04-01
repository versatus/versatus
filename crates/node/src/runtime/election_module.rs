use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap},
    error::Error,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use async_trait::async_trait;
use block::{header::BlockHeader, Conflict, ConflictList, RefHash, ResolvedConflicts};
use ethereum_types::U256;
use events::{ConflictBytes, DirectedEvent, Event, Topic};
use primitives::NodeId;
use quorum::{election::Election, quorum::Quorum, Quorum};
use serde::{Deserialize, Serialize};
use sha256::digest;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use vrrb_core::{
    claim::Claim,
    event_router::{DirectedEvent, Event},
};

use crate::NodeError;

pub type Seed = u64;

pub trait ElectionType: Clone + Debug {}
pub trait ElectionOutcome: Clone + Debug {}

pub type MinerElectionResult = Vec<ElectionResult>;
pub type QuorumElectionResult = HashMap<u8, Vec<ElectionResult>>;
pub type ConflictResolutionResult = HashMap<String, ElectionResult>;

#[derive(Clone, Debug)]
pub struct MinerElection;

#[derive(Clone, Debug)]
pub struct QuorumElection;

#[derive(Clone, Debug)]
pub struct ConflictResolution;

pub struct ElectionModuleConfig {
    pub db_read_handle: VrrbDbReadHandle,
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    pub local_claim: Claim,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElectionResult {
    pub claim_pointer: u128,
    pub claim_hash: String,
    pub node_id: NodeId,
}

#[derive(Clone, Debug)]
pub struct ElectionModule<E, T>
where
    E: ElectionType,
    T: ElectionOutcome,
{
    election_type: E,
    status: ActorState,
    id: ActorId,
    label: ActorLabel,
    pub db_read_handle: VrrbDbReadHandle,
    pub local_claim: Claim,
    pub outcome: Option<T>,
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

impl ElectionModule<MinerElection, MinerElectionResult> {
    pub fn new(config: ElectionModuleConfig) -> ElectionModule<MinerElection, MinerElectionResult> {
        ElectionModule {
            election_type: MinerElection,
            status: ActorState::Stopped,
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("Election module"),
            db_read_handle: config.db_read_handle,
            local_claim: config.local_claim,
            outcome: None,
            events_tx: config.events_tx,
        }
    }

    pub fn name(&self) -> ActorLabel {
        String::from("Miner Election Module")
    }
}

impl ElectionModule<QuorumElection, QuorumElectionResult> {
    pub fn new(
        config: ElectionModuleConfig,
    ) -> ElectionModule<QuorumElection, QuorumElectionResult> {
        ElectionModule {
            election_type: QuorumElection,
            status: ActorState::Stopped,
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("Election module"),
            db_read_handle: config.db_read_handle,
            local_claim: config.local_claim,
            outcome: None,
            events_tx: config.events_tx,
        }
    }

    pub fn name(&self) -> ActorLabel {
        String::from("Quorum Election Module")
    }
}

impl ElectionModule<ConflictResolution, ConflictResolutionResult> {
    pub fn new(
        config: ElectionModuleConfig,
    ) -> ElectionModule<ConflictResolution, ConflictResolutionResult> {
        ElectionModule {
            election_type: ConflictResolution,
            status: ActorState::Stopped,
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("Election module"),
            db_read_handle: config.db_read_handle,
            local_claim: config.local_claim,
            outcome: None,
            events_tx: config.events_tx,
        }
    }

    pub fn name(&self) -> ActorLabel {
        String::from("Conflict Resultion Election Module")
    }
}


impl ElectionType for MinerElection {}
impl ElectionType for QuorumElection {}
impl ElectionType for ConflictResolution {}

impl ElectionOutcome for MinerElectionResult {}
impl ElectionOutcome for QuorumElectionResult {}
impl ElectionOutcome for ConflictResolutionResult {}

#[async_trait]
impl Handler<Event> for ElectionModule<MinerElection, MinerElectionResult> {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::MinerElection(header_bytes) => {
                let header_result: Result<BlockHeader> = serde_json::from_slice(&header_bytes);
                if let Ok(header) = header_result {
                    let claims = self.db_read_handle.claim_store_values();
                    let mut election_results: BTreeMap<U256, String> =
                        elect_miner(claims, header.block_seed);

                    let winner = get_winner(&mut election_results);

                    let directed_event = (Topic::Consensus, Event::ElectedMiner(winner));
                    let _ = self.events_tx.send(directed_event);
                }
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[async_trait]
impl Handler<Event> for ElectionModule<QuorumElection, QuorumElectionResult> {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::QuorumElection(kp, last_block_height) => {
                let claims = self.db_read_handle.claim_store_values();
                let mut hasher = DefaultHasher::new();
                kp.get_miner_public_key().hash(&mut hasher);
                let pubkey_hash = hasher.finish();

                let mut pub_key_bytes = pubkey_hash.to_string().as_bytes().to_vec();
                pub_key_bytes.push(1u8);

                let hash = digest(digest(&*pub_key_bytes).as_bytes());
                let payload = (10, hash);

                if let Ok(seed) = Quorum::generate_seed(payload, kp.clone()) {
                    if let Ok(mut quorum) = Quorum::new(seed, last_block_height, kp.clone()) {
                        if let Ok(elected_quorum) =
                            quorum.run_election(claims.values().cloned().collect::<Vec<Claim>>())
                        {
                            let directed_event = Event::ElectedQuorum(elected_quorum.clone());
                            let _ = self.events_tx.send(directed_event);
                        }
                    }
                }
            },

            _ => {},
        }

        Ok(ActorState::Running)
    }
}

#[async_trait]
impl Handler<Event> for ElectionModule<ConflictResolution, ConflictResolutionResult> {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::ConflictResolution(ConflictBytes, HeaderBytes) => {
                let cl_res: Result<ConflictList> = serde_json::from_slice(&ConflictBytes);

                let header_res: Result<BlockHeader> = serde_json::from_slice(&HeaderBytes);

                if let Ok(conflicts) = cl_res {
                    if let Ok(header) = header_res {
                        let handles: ResolvedConflicts = conflicts
                            .iter()
                            .map(|(txnid, conflict)| {
                                let inner_header = header.clone();
                                let events_tx = self.events_tx.clone();
                                let mut inner_conflict: Conflict = conflict.clone();
                                tokio::spawn(async move {
                                    resolve_conflict(
                                        &mut inner_conflict,
                                        inner_header.clone(),
                                        events_tx,
                                    )
                                    .await;
                                });
                            })
                            .collect();
                    }
                }
            },
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

fn elect_miner(claims: HashMap<NodeId, Claim>, block_seed: u64) -> BTreeMap<U256, NodeId> {
    claims
        .iter()
        .filter(|(_, claim)| claim.eligible)
        .map(|(nodeid, claim)| single_miner_results(claim, nodeid, block_seed))
        .collect()
}

fn single_miner_results(claim: Claim, node_id: NodeId, block_seed: u64) -> (U256, NodeId) {
    (claim.get_election_result(block_seed), node_id)
}

fn get_winner(results: &mut BTreeMap<U256, NodeId>) -> (U256, NodeId) {
    let mut first: Option<(U256, NodeId)> = election_results.pop_first();
    while let None = first {
        first = election_results.pop_first();
    }

    return first;
}

async fn resolve_conflict(
    conflict: &mut Conflict,
    header: BlockHeader,
    events_tx: UnboundedSender<DirectedEvent>,
) {
    let propopsers = conflict.proposers.clone();
    let resoultion_results: BTreeMap<U256, String> = proposers
        .iter()
        .map(|(claim, refhash)| {
            (
                claim.get_election_results(inner_header.block_seed.clone()),
                refhash.clone(),
            );
        })
        .collect();

    let winner = {
        let mut first: Option<(U256, NodeId)> = resolution_results.pop_first();

        while let None = first {
            first = resolution_results.pop_first();
        }
        first
    };

    conflict.winner = Some(winner.1);
    let directed_event = (Topic::Consensus, Event::ConflictResolved(conflict));
    let _ = events_tx.send(directed_event);
}
