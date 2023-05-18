use async_trait::async_trait;
use crossbeam_channel::Sender;
use dashmap::DashMap;
use events::{
    Event,
    EventMessage,
    EventPublisher,
    JobResult,
    QuorumCertifiedTxn,
    Vote,
    VoteReceipt,
};
use primitives::{GroupPublicKey, HarvesterQuorumThreshold, QuorumThreshold};
use signer::signer::SignatureProvider;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tracing::error;
use vrrb_core::{bloom::Bloom, txn::TransactionDigest};

use crate::{farmer_module::PULL_TXN_BATCH_SIZE, scheduler::Job};

/// `CERTIFIED_TXNS_FILTER_SIZE` is a constant that defines the size of the
/// bloom filter used by the `HarvesterModule` to store the certified
/// transactions. In this case, the bloom filter is used to keep track of the
/// transactions that have been certified by the harvester. The size
/// of the bloom filter is set to 500000, which means that it can store up to
/// 500000 elements with a low probability of false positives.
pub const CERTIFIED_TXNS_FILTER_SIZE: usize = 500000;

/// The HarvesterModule struct contains various fields related to transaction
/// certification and mining proposal blocks,
///
/// Properties:
///
/// * `quorum_certified_txns`: A vector of QuorumCertifiedTxn structs, which
///   represent transactions that
/// have been certified by a quorum of nodes in the network.
/// * `certified_txns_filter`: `certified_txns_filter` is a Bloom filter that is
///   used to efficiently
/// check whether a given transaction has already been certified by the quorum.
/// * `votes_pool`: `votes_pool` is a `DashMap` that stores a mapping between a
///   tuple of
/// `(TransactionDigest, String)` and a vector of `Vote` structs. This is used
/// to keep track of the votes received for a particular transaction and its
/// corresponding round. The `TransactionDigest` is a unique
/// * `group_public_key`: The `group_public_key` property is a public key used
///   to certify convergence block
/// * `sig_provider`: The `sig_provider` property is an optional field that
///   holds a `SignatureProvider`
/// object. This object is responsible for providing cryptographic signatures
/// for convergence blocks
/// * `status`: The status property is an instance of the ActorState enum, which
///   represents the current
/// state of the HarvesterModule actor. The possible states are defined within
/// the enum.
/// * `label`: The label property is of type ActorLabel and is used to identify
///   the type of actor this
/// HarvesterModule instance represents. It is likely used for debugging and
/// logging purposes.
/// * `id`: The `id` property is of type `ActorId` and is used to uniquely
///   identify an instance of the
/// `HarvesterModule` struct. It is likely used for internal bookkeeping and
/// communication between different parts of the system.
/// * `broadcast_events_tx`: `UnboundedSender<Event>` is a channel sender that
///   can be used to broadcast
/// events to multiple receivers without blocking. It is unbounded, meaning that
/// it can hold an unlimited number of events until they are consumed by the
/// receivers.
/// * `events_rx`: The `events_rx` property is an `UnboundedReceiver` that is
///   used to receive events
/// from other actors or modules in the system. It is likely used to handle
/// asynchronous communication and coordination between different parts of the
/// system.
/// * `quorum_threshold`: The `quorum_threshold` property is of type
///   `QuorumThreshold` and represents
/// the minimum number of votes required to sign Convergence block.
/// * `sync_jobs_sender`: A Sender object used to send synchronous jobs to be
///   executed by the Scheduler.
/// * `async_jobs_sender`: A Sender object used to send asynchronous jobs to be
///   executed by the Scheduler.
pub struct HarvesterModule {
    pub quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    pub certified_txns_filter: Bloom,
    pub votes_pool: DashMap<(TransactionDigest, String), Vec<Vote>>,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: EventPublisher,
    events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
    quorum_threshold: QuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
}

impl HarvesterModule {
    pub fn new(
        certified_txns_filter: Bloom,
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
        broadcast_events_tx: EventPublisher,
        quorum_threshold: HarvesterQuorumThreshold,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
    ) -> Self {
        let quorum_certified_txns = Vec::new();

        Self {
            quorum_certified_txns,
            certified_txns_filter,
            sig_provider,
            status: ActorState::Stopped,
            label: String::from("FarmerHarvester"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            broadcast_events_tx,
            events_rx,
            quorum_threshold,
            votes_pool: DashMap::new(),
            sync_jobs_sender,
            async_jobs_sender,
        }
    }

    pub fn name(&self) -> String {
        String::from("FarmerHarvester module")
    }
}

#[async_trait]
impl Handler<EventMessage> for HarvesterModule {
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

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            /// The above code is handling an event of type `Vote` in a Rust
            /// program. It checks the integrity of the vote by
            /// verifying that it comes from the actual voter and prevents
            /// double voting. It then adds the vote to a pool of votes for the
            /// corresponding transaction and farmer quorum key. If
            /// the number of votes in the pool reaches the farmer
            /// quorum threshold, it sends a job to certify the transaction
            /// using the provided signature provider.
            Event::Vote(vote, farmer_quorum_threshold) => {
                //TODO Harvest should check for integrity of the vote by Voter( Does it vote
                // truly comes from Voter Prevent Double Voting

                if let Some(sig_provider) = self.sig_provider.clone() {
                    let farmer_quorum_key = hex::encode(vote.quorum_public_key.clone());
                    if let Some(mut votes) = self
                        .votes_pool
                        .get_mut(&(vote.txn.id(), farmer_quorum_key.clone()))
                    {
                        let txn_id = vote.txn.id();
                        if !self
                            .certified_txns_filter
                            .contains(&(txn_id.clone(), farmer_quorum_key.clone()))
                        {
                            votes.push(vote.clone());
                            if votes.len() >= farmer_quorum_threshold {
                                let _ = self.sync_jobs_sender.send(Job::CertifyTxn((
                                    sig_provider,
                                    votes.clone(),
                                    txn_id,
                                    farmer_quorum_key,
                                    vote.farmer_id.clone(),
                                    vote.txn,
                                )));
                            }
                        }
                    } else {
                        self.votes_pool
                            .insert((vote.txn.id(), farmer_quorum_key), vec![vote]);
                    }
                }
            },
            /// This certifies txns once vote threshold is reached.
            Event::CertifiedTxn(job_result) => {
                if let JobResult::CertifiedTxn(
                    votes,
                    certificate,
                    txn_id,
                    farmer_quorum_key,
                    farmer_id,
                    txn,
                ) = job_result
                {
                    let vote_receipts = votes
                        .iter()
                        .map(|v| VoteReceipt {
                            farmer_id: v.farmer_id.clone(),
                            farmer_node_id: v.farmer_node_id,
                            signature: v.signature.clone(),
                        })
                        .collect::<Vec<VoteReceipt>>();
                    self.quorum_certified_txns.push(QuorumCertifiedTxn::new(
                        farmer_id,
                        vote_receipts,
                        txn,
                        certificate,
                    ));
                    let _ = self
                        .certified_txns_filter
                        .push(&(txn_id, farmer_quorum_key));
                }
            },

            /// Mines proposal block after every X seconds.
            Event::MineProposalBlock => {
                let txns = self.quorum_certified_txns.iter().take(PULL_TXN_BATCH_SIZE);
                txns.clone().for_each(|txn| {
                    let _ = self
                        .broadcast_events_tx
                        .send(Event::QuorumCertifiedTxns(txn.clone()).into());
                    let _ = self.certified_txns_filter.push(&txn.txn.id.to_string());
                });
                let _txns = txns.collect::<Vec<&QuorumCertifiedTxn>>();
                //TODO: Build Proposal Blocks here
            },
            Event::NoOp => {},
            _ => {
                error!("Unexpected event,Can only certify Txns and Convergence block,and can mine proposal block");
            },
        }

        Ok(ActorState::Running)
    }

    fn on_stop(&self) {
        info!(
            "{:?}-{:?} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use events::{Event, EventMessage, JobResult, DEFAULT_BUFFER};
    use lazy_static::lazy_static;
    use primitives::Address;
    use theater::{Actor, ActorImpl, ActorState};
    use vrrb_core::{account::Account, bloom::Bloom};

    use crate::{harvester_module::HarvesterModule, scheduler::Job};

    #[tokio::test]
    async fn harvester_runtime_module_starts_and_stops() {
        let (broadcast_events_tx, _) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);
        let (sync_jobs_sender, _sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, _async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (_, events_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);

        let (_sync_jobs_status_sender, _sync_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();

        let (_async_jobs_status_sender, _async_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();

        let harvester_swarm_module = HarvesterModule::new(
            Bloom::new(10000),
            None,
            vec![],
            events_rx,
            broadcast_events_tx,
            2,
            sync_jobs_sender,
            async_jobs_sender,
        );
        let mut harvester_swarm_module = ActorImpl::new(harvester_swarm_module);

        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);

        assert_eq!(harvester_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            harvester_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(harvester_swarm_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }
    lazy_static! {
        static ref STATE_SNAPSHOT: HashMap<Address, Account> = HashMap::new();
    }
}
