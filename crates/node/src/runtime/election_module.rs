use std::collections::{HashMap, BTreeMap};
use block::header::BlockHeader;
use telemetry::info;
use async_trait::async_trait;
use primitives::NodeId;
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{ActorId, ActorState, ActorLabel, Handler};
use vrrb_core::{claim::Claim, event_router::{DirectedEvent, Event}};
use serde::{Serialize, Deserialize};
use std::fmt::Debug;

// TODO: Create Seed Struct that 
// checks upon creation that the 
// value of the seed is between 
// u32::MAX and u64::MAX 
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
    pub fn new(
        config: ElectionModuleConfig
    ) -> ElectionModule<MinerElection, MinerElectionResult> 
    {
        ElectionModule {
            election_type: MinerElection,
            status: ActorState::Stopped,
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("State module"),
            db_read_handle: config.db_read_handle,
            local_claim: config.local_claim,
            outcome: None,
            events_tx: config.events_tx
        }
    }

    pub fn name(&self) -> ActorLabel {
        String::from("Miner Election Module") 
    }
}

impl ElectionModule<QuorumElection, QuorumElectionResult> {
    pub fn new(
        config: ElectionModuleConfig
    ) -> ElectionModule<QuorumElection, QuorumElectionResult> {
        ElectionModule { 
            election_type: QuorumElection, 
            status: ActorState::Stopped, 
            id: uuid::Uuid::new_v4().to_string(), 
            label: String::from("State module"), 
            db_read_handle: config.db_read_handle, 
            local_claim: config.local_claim, 
            outcome: None, 
            events_tx: config.events_tx 
        } 
    }

    pub fn name(&self) -> ActorLabel {
        String::from("Quorum Election Module") 
    }
}

impl ElectionModule<ConflictResolution, ConflictResolutionResult> {
    pub fn new(
        config: ElectionModuleConfig 
    ) -> ElectionModule<ConflictResolution, ConflictResolutionResult> {
        ElectionModule { 
            election_type: ConflictResolution, 
            status: ActorState::Stopped, 
            id: uuid::Uuid::new_v4().to_string(), 
            label: String::from("State module"), 
            db_read_handle: config.db_read_handle, 
            local_claim: config.local_claim, 
            outcome: None, 
            events_tx: config.events_tx 
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
                let header_result: Result<BlockHeader> = serde_json::from_slice(
                    &header_bytes
                );
                if let Ok(header) = header_result {
                    let claims = self.db_read_handle.claim_store_values();
                    let pointer_sums: BTreeMap<Option<u128>, String> = claims.iter()
                        .filter(|(_, claim)| claim.eligible)
                        .map(
                            |(nodeid, claim)| {
                                (claim.get_pointer(
                                    header.next_block_seed
                                ), node_id.clone())
                        }
                    ).collect();
                    
                    let ps_iter = pointer_sums.iter();
                    let winner = {
                        let mut tmp: (u128, NodeId);
                        while let Some((ps, node_id)) = ps_iter.next() {
                            if ps.is_some() { 
                                tmp = (*ps, node_id.to_string());
                                break
                            }
                        }
                       
                        tmp
                    };
                }
            }
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
            //TODO: Implement
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
            //TODO: Implement
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
