use std::{
    borrow::{Borrow, BorrowMut},
    thread,
};

use async_trait::async_trait;
use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;
use events::{DirectedEvent, Event, QuorumCertifiedTxn, Topic, Vote, VoteReceipt};
use lr_trie::ReadHandleFactory;
use mempool::mempool::{LeftRightMempool, TxnStatus};
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::{
    FarmerQuorumThreshold,
    GroupPublicKey,
    HarvesterQuorumThreshold,
    NodeIdx,
    PeerId,
    QuorumThreshold,
    QuorumType,
    RawSignature,
};
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use signer::signer::{SignatureProvider, Signer};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::{
    broadcast::error::TryRecvError,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::error;
use vrrb_core::{
    bloom::Bloom,
    txn::{TransactionDigest, Txn},
};

use crate::{
    farmer_module::PULL_TXN_BATCH_SIZE,
    result::Result,
    scheduler::{Job, JobResult},
    NodeError,
};


/// `HarvesterModule` is responsible for
/// 1. for collecting votes from farmers and then certify the Txns
/// 2. Certify convergence block
///
/// Properties:
///
/// * `quorum_certified_txns`: This is a vector of QuorumCertifiedTxn structs.
///   This is the list of
/// transactions that have been certified by the harvester.
/// * `certified_txns_filter`: A bloom filter that contains all the transactions
///   that have been
/// certified by the harvester.
/// * `tx_mempool`: This is the mempool that the harvester uses to store
///   transactions.
/// * `votes_pool`: This is a map of all the votes that have been received for a
///   given transaction.
/// * `group_public_key`: The public key of the group that the harvester is
///   harvesting for.
/// * `sig_provider`: This is the signature provider that will be used to sign
///   the transactions.
/// * `status`: ActorState - the state of the actor
/// * `label`: The label of the actor.
/// * `id`: ActorId - The id of the actor.
/// * `broadcast_events_tx`: This is the channel that the HarvesterModule uses
///   to send events to the
/// EventHandler.
/// * `clear_filter_rx`: This is a channel that the Harvester listens on for a
///   message to clear the
/// certified_txns_filter.
/// * `quorum_threshold`: QuorumThreshold - The threshold of votes required to
///   certify a transaction.
/// * `sync_jobs_sender`: Sender<Job> - This is a channel that the
///   HarvesterModule uses to send jobs to
/// the SyncWorker.
/// * `async_jobs_sender`: Sender<Job>
/// * `sync_jobs_status_receiver`: Receiver<JobResult>
/// * `async_jobs_status_receiver`: Receiver<JobResult>
pub struct HarvesterModule {
    pub quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    pub certified_txns_filter: Bloom,
    pub tx_mempool: Option<LeftRightMempool>,
    pub votes_pool: DashMap<(TransactionDigest, String), Vec<Vote>>,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: UnboundedSender<DirectedEvent>,
    clear_filter_rx: UnboundedReceiver<DirectedEvent>,
    quorum_threshold: QuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
    sync_jobs_status_receiver: Receiver<JobResult>,
    async_jobs_status_receiver: Receiver<JobResult>,
}


impl HarvesterModule {
    pub fn new(
        certified_txns_filter: Bloom,
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        broadcast_events_tx: UnboundedSender<DirectedEvent>,
        clear_filter_rx: UnboundedReceiver<DirectedEvent>,
        quorum_threshold: HarvesterQuorumThreshold,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
        sync_jobs_status_receiver: Receiver<JobResult>,
        async_jobs_status_receiver: Receiver<JobResult>,
    ) -> Self {
        let lrmpooldb = Some(LeftRightMempool::new());

        let quorum_certified_txns = Vec::new();
        let harvester = Self {
            quorum_certified_txns,
            certified_txns_filter,
            sig_provider,
            tx_mempool: lrmpooldb,
            status: ActorState::Stopped,
            label: String::from("FarmerHarvester"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            broadcast_events_tx: broadcast_events_tx.clone(),
            clear_filter_rx,
            quorum_threshold,
            votes_pool: DashMap::new(),
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver: sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver: async_jobs_status_receiver.clone(),
        };
        harvester
    }

    fn process_sync_job_status(&mut self, sync_jobs_status_receiver: Receiver<JobResult>) {
        loop {
            let job_result = sync_jobs_status_receiver.recv().unwrap();
            match job_result {
                JobResult::CertifiedTxn(
                    votes,
                    certificate,
                    txn_id,
                    farmer_quorum_key,
                    farmer_id,
                    txn,
                ) => {
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
                },
                _ => {
                    error!("Harvester can only certify Txn and Convergence block, and can mine proposal block")
                },
            }
        }
    }

    pub fn insert_txn(&mut self, txn: Txn) {
        if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
            let _ = tx_mempool.insert(txn);
        }
    }

    pub fn update_txn_status(&mut self, txn_id: TransactionDigest, status: TxnStatus) {
        if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
            let txn_record_opt = tx_mempool.get(&txn_id);
            if let Some(mut txn_record) = txn_record_opt {
                txn_record.status = status;
                self.remove_txn(txn_id);
                self.insert_txn(txn_record.txn);
            }
        }
    }

    pub fn remove_txn(&mut self, txn_id: TransactionDigest) {
        if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
            let _ = tx_mempool.remove(&txn_id);
        }
    }

    pub fn name(&self) -> String {
        String::from("FarmerHarvester module")
    }
}

#[async_trait]
impl Handler<Event> for HarvesterModule {
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
            "{:?}-{:?} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::Vote(vote, quorum, farmer_quorum_threshold) => {
                //Harvest should check for integrity of the vote by Voter( Does it vote truly
                // comes from Voter Prevent Double Voting
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
                                    sig_provider.clone(),
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
            Event::PullQuorumCertifiedTxns(num_of_txns) => {
                self.quorum_certified_txns
                    .iter()
                    .take(num_of_txns)
                    .for_each(|txn| {
                        self.broadcast_events_tx
                            .send((Topic::Storage, Event::QuorumCertifiedTxns(txn.clone())));
                    });
            },
            Event::NoOp => {},
            _ => {
                error!("Unexpected event,Can only certify Txns and Convergence block,and can mine proposal block");
            },
        }

        Ok(ActorState::Running)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        env,
        net::{IpAddr, Ipv4Addr},
        process::exit,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use dkg_engine::{test_utils, types::config::ThresholdConfig};
    use events::{DirectedEvent, Event, PeerData, Vote};
    use lazy_static::lazy_static;
    use primitives::{NodeType, QuorumType::Farmer};
    use secp256k1::Message;
    use theater::ActorImpl;
    use validator::{
        txn_validator::{StateSnapshot, TxnValidator},
        validator_core_manager::ValidatorCoreManager,
    };
    use vrrb_core::{cache, is_enum_variant, keypair::KeyPair, txn::NewTxnArgs};

    use super::*;
    use crate::scheduler::JobSchedulerController;

    #[tokio::test]
    async fn harvester_runtime_module_starts_and_stops() {
        let (broadcast_events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

        let (sync_jobs_status_sender, sync_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let (async_jobs_status_sender, async_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let harvester_swarm_module = HarvesterModule::new(
            Bloom::new(10000),
            None,
            vec![],
            broadcast_events_tx,
            clear_filter_rx,
            2,
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver.clone(),
        );
        let mut harvester_swarm_module = ActorImpl::new(harvester_swarm_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(harvester_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            harvester_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(harvester_swarm_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }
    lazy_static! {
        static ref STATE_SNAPSHOT: StateSnapshot = StateSnapshot {
            accounts: HashMap::new(),
        };
    }
}
