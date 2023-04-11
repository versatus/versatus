use std::collections::BTreeMap;

use crossbeam_channel::Receiver;
use events::{DirectedEvent, Event, JobResult, Vote};
use job_scheduler::JobScheduler;
use mempool::TxnRecord;
use primitives::{base::PeerId as PeerID, ByteVec, FarmerQuorumThreshold};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use signer::signer::{SignatureProvider, Signer};
use storage::vrrbdb::VrrbDbReadHandle;
use tokio::sync::mpsc::UnboundedSender;
use tracing::error;
use validator::validator_core_manager::ValidatorCoreManager;
use vrrb_core::txn::{TransactionDigest, Txn};


/// The `JobSchedulerController` struct contains various fields related to job
/// scheduling
///
/// Properties:
///
/// * `job_scheduler`: A property of type `JobScheduler`, which is likely a
///   struct or class that manages
/// the scheduling and execution of jobs.
/// * `events_tx`: `events_tx` is a variable of type
///   `UnboundedSender<DirectedEvent>`. It is used to
/// send `DirectedEvent` messages to the event loop of the job scheduler. The
/// `UnboundedSender` type is a channel sender that can send an unlimited number
/// of messages without blocking.
/// * `sync_jobs_receiver`: `sync_jobs_receiver` is a `Receiver` that receives
///   synchronous jobs. A
/// `Receiver` is a channel receiver used for receiving messages from a channel.
/// In this case, it is used to receive `Job` objects that need to be executed
/// synchronously.
/// * `async_jobs_receiver`: `async_jobs_receiver` is a `Receiver` that receives
///   `Job` objects
/// asynchronously. It is likely used to receive jobs that can be executed in
/// parallel with other jobs, without blocking the main thread.
/// * `validator_core_manager`: `validator_core_manager` is a property of type
///   `ValidatorCoreManager`.
/// It is likely a struct or an enum that manages the core functionality of the
/// validator, such as validating transactions and blocks, maintaining the
/// blockchain state, and communicating with other nodes in the network. It may
/// also handle consensus algorithms and
/// * `vrrbdb_read_handle`: `vrrbdb_read_handle` is a handle to read data from
///   the VRRB database.
pub struct JobSchedulerController {
    pub job_scheduler: JobScheduler,
    events_tx: UnboundedSender<Event>,
    sync_jobs_receiver: Receiver<Job>,
    async_jobs_receiver: Receiver<Job>,
    pub validator_core_manager: ValidatorCoreManager,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
}

pub enum Job {
    /// `Farm` is an enum variant of the `Job` enum. Its job is to vote on Txns
    /// within the mempool and share it across harvesters
    Farm(
        (
            Vec<(TransactionDigest, TxnRecord)>,
            ByteVec,
            u16,
            ByteVec,
            SignatureProvider,
            FarmerQuorumThreshold,
        ),
    ),
    /// `CertifyTxn` is an enum variant of the `Job` enum. It represents a job
    /// to certify a transaction. The job takes in a tuple containing a
    /// `SignatureProvider`, a vector of `Vote` objects, a
    /// `TransactionDigest`, a `String` representing the farmer quorum key, a
    /// `Vec<u8>` representing the farmer ID, and a `Txn` object
    /// representing the transaction to be certified.
    CertifyTxn(
        (
            SignatureProvider,
            Vec<Vote>,
            TransactionDigest,
            String,
            Vec<u8>,
            Txn,
        ),
    ),
}


impl<'a> JobSchedulerController {
    pub fn new(
        peer_id: PeerID,
        events_tx: UnboundedSender<Event>,
        sync_jobs_receiver: Receiver<Job>,
        async_jobs_receiver: Receiver<Job>,
        validator_core_manager: ValidatorCoreManager,
        vrrbdb_read_handle: VrrbDbReadHandle,
    ) -> Self {
        Self {
            job_scheduler: JobScheduler::new(peer_id),
            events_tx,
            sync_jobs_receiver,
            async_jobs_receiver,
            validator_core_manager,
            vrrbdb_read_handle,
        }
    }

    pub fn execute_sync_jobs(&mut self) {
        loop {
            match self.sync_jobs_receiver.try_recv() {
                Ok(job) => match job {
                    Job::Farm((
                        txns,
                        receiver_farmer_id,
                        farmer_node_id,
                        quorum_public_key,
                        sig_provider,
                        farmer_quorum_threshold,
                    )) => {
                        let transactions: Vec<Txn> = txns.iter().map(|x| x.1.txn.clone()).collect();
                        let validated_txns: Vec<_> = self
                            .validator_core_manager
                            .validate(&self.vrrbdb_read_handle.state_store_values(), transactions)
                            .into_iter()
                            .collect();
                        let backpressure = self.job_scheduler.calculate_back_pressure();
                        //Delegation Principle need to be done
                        let votes_result = self
                            .job_scheduler
                            .get_local_pool()
                            .run_sync_job(move || {
                                let votes = validated_txns
                                    .par_iter()
                                    .map_with(
                                        receiver_farmer_id,
                                        |receiver_farmer_id: &mut Vec<u8>, txn| {
                                            let mut vote = None;
                                            let txn = txn.0.clone();
                                            if let Ok(txn_bytes) = bincode::serialize(&txn) {
                                                if let Ok(signature) = sig_provider
                                                    .generate_partial_signature(txn_bytes)
                                                {
                                                    vote = Some(Vote {
                                                        farmer_id: receiver_farmer_id.clone(),
                                                        farmer_node_id,
                                                        signature,
                                                        txn,
                                                        quorum_public_key: quorum_public_key
                                                            .clone(),
                                                        quorum_threshold: 2,
                                                        execution_result: None,
                                                    });
                                                }
                                            }
                                            vote
                                        },
                                    )
                                    .collect::<Vec<Option<Vote>>>();
                                votes
                            })
                            .join();
                        if let Ok(votes) = votes_result {
                            let _ = self.events_tx.send(Event::ProcessedVotes(JobResult::Votes((
                                votes,
                                farmer_quorum_threshold,
                            ))));
                        }
                    },
                    Job::CertifyTxn((
                        sig_provider,
                        votes,
                        txn_id,
                        farmer_quorum_key,
                        farmer_id,
                        txn,
                    )) => {
                        let mut sig_shares = BTreeMap::new();
                        for v in votes.iter() {
                            sig_shares.insert(v.farmer_node_id, v.signature.clone());
                        }
                        let validated_txns: Vec<_> = self
                            .validator_core_manager
                            .validate(
                                &self.vrrbdb_read_handle.state_store_values(),
                                vec![txn.clone()],
                            )
                            .into_iter()
                            .collect();
                        let validated = validated_txns.par_iter().any(|x| x.0.id() == txn.id());
                        if validated {
                            let result = sig_provider.generate_quorum_signature(sig_shares.clone());
                            if let Ok(threshold_signature) = result {
                                let _ = self.events_tx.send(Event::CertifiedTxn(
                                    JobResult::CertifiedTxn(
                                        votes.clone(),
                                        threshold_signature,
                                        txn_id.clone(),
                                        farmer_quorum_key.clone(),
                                        farmer_id.clone(),
                                        txn.clone(),
                                    ),
                                ));
                            } else {
                                error!("Quorum signature generation failed");
                            }
                        } else {
                            error!("Penalize Farmer for wrong votes by sending Wrong Vote event to CR Quorum");
                        }
                    },
                },
                Err(_) => {},
            }
        }
    }
}
