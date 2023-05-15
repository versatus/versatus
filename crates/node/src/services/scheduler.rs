use std::collections::{BTreeMap, HashMap};

use block::ConvergenceBlock;
use crossbeam_channel::Receiver;
use events::{Event, EventPublisher, JobResult, Vote};
use hbbft::crypto::SignatureShare;
use job_scheduler::JobScheduler;
use mempool::TxnRecord;
use primitives::{base::PeerId as PeerID, ByteVec, FarmerQuorumThreshold, NodeIdx};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use signer::signer::{SignatureProvider, Signer};
use storage::vrrbdb::VrrbDbReadHandle;
use tracing::error;
use validator::validator_core_manager::ValidatorCoreManager;
use vrrb_core::txn::{TransactionDigest, Txn};

/// The `JobSchedulerController` to manage `JobScheduler`,
/// Properties:
///
/// * `job_scheduler`: A property of type `JobScheduler`, which is likely a
///   struct or class that manages
/// scheduling and execution of jobs.
/// * `events_tx`: `events_tx` is an `UnboundedSender` that is used to send
///   events to the
/// `JobSchedulerController`. It is a channel sender that can send an unlimited
/// number of messages without blocking. This is useful for sending events
/// asynchronously to the controller.
/// * `sync_jobs_receiver`: `sync_jobs_receiver` is a `Receiver` that receives
///   synchronous jobs. A
/// `Receiver` is a channel receiver that can be used to receive values sent by
/// a corresponding `Sender`. In this case, it is used to receive `Job` objects
/// that need to be executed synchronously.
/// * `async_jobs_receiver`: `async_jobs_receiver` is a `Receiver` that receives
///   `Job` objects
/// asynchronously. It is likely used in conjunction with the `JobScheduler` to
/// handle and execute jobs in a concurrent manner.
/// * `validator_core_manager`: `validator_core_manager` is a property of type
///   `ValidatorCoreManager`.
/// It is likely a struct or a class that manages the core functionality of a
/// validator. This could include tasks such as validating transactions,
/// managing the state of the blockchain, and communicating with other nodes in
/// the network. The `Job
/// * `vrrbdb_read_handle`: VrrbDbReadHandle is likely a handle or reference to
///   a database read
/// operation for a specific database. It could be used by the
/// JobSchedulerController to retrieve data from the database as needed for its
/// job scheduling tasks. The specific implementation and functionality of
/// VrrbDbReadHandle would depend on
pub struct JobSchedulerController {
    pub job_scheduler: JobScheduler,
    events_tx: EventPublisher,
    sync_jobs_receiver: Receiver<Job>,
    async_jobs_receiver: Receiver<Job>,
    pub validator_core_manager: ValidatorCoreManager,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
}

pub enum Job {
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
    CertifyTxn(
        (
            SignatureProvider,
            Vec<Vote>,
            TransactionDigest,
            String,
            Vec<u8>,
            Txn,
            FarmerQuorumThreshold,
        ),
    ),
    SignConvergenceBlock(SignatureProvider, ConvergenceBlock),
}

impl JobSchedulerController {
    pub fn new(
        peer_id: PeerID,
        events_tx: EventPublisher,
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

                        //TODO  Add Delegation logic + Handling Double Spend by checking whether
                        // MagLev Hashing over( Quorum Keys) to identify whether current farmer
                        // quorum is supposed to vote on txn Txn is intended
                        // to be validated by current validator
                        let _backpressure = self.job_scheduler.calculate_back_pressure();
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
                                            let new_txn = txn.0.clone();
                                            if let Ok(txn_bytes) = bincode::serialize(&new_txn) {
                                                if let Ok(signature) = sig_provider
                                                    .generate_partial_signature(txn_bytes)
                                                {
                                                    vote = Some(Vote {
                                                        farmer_id: receiver_farmer_id.clone(),
                                                        farmer_node_id,
                                                        signature,
                                                        txn: new_txn,
                                                        quorum_public_key: quorum_public_key
                                                            .clone(),
                                                        quorum_threshold: farmer_quorum_threshold,
                                                        execution_result: None,
                                                        is_txn_valid: txn.1.is_err(),
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
                            let _ = self.events_tx.send(
                                Event::ProcessedVotes(
                                    JobResult::Votes((votes, farmer_quorum_threshold)).into(),
                                )
                                .into(),
                            );
                        }
                    },
                    Job::CertifyTxn((
                        sig_provider,
                        votes,
                        txn_id,
                        farmer_quorum_key,
                        farmer_id,
                        txn,
                        farmer_quorum_threshold,
                    )) => {
                        let mut vote_shares: HashMap<bool, BTreeMap<NodeIdx, Vec<u8>>> =
                            HashMap::new();
                        for v in votes.iter() {
                            if let Some(votes) = vote_shares.get_mut(&v.is_txn_valid) {
                                votes.insert(v.farmer_node_id, v.signature.clone());
                            } else {
                                let sig_shares_map: BTreeMap<NodeIdx, Vec<u8>> =
                                    vec![(v.farmer_node_id, v.signature.clone())]
                                        .into_iter()
                                        .collect();
                                vote_shares.insert(v.is_txn_valid, sig_shares_map);
                            }
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
                        let most_votes_share = vote_shares
                            .iter()
                            .max_by_key(|(_, votes_map)| votes_map.len())
                            .map(|(key, votes_map)| (*key, votes_map.clone()));
                        if validated {
                            if let Some((is_txn_valid, votes_map)) = most_votes_share {
                                let result = sig_provider.generate_quorum_signature(
                                    farmer_quorum_threshold as u16,
                                    votes_map.clone(),
                                );
                                if let Ok(threshold_signature) = result {
                                    let _ = self.events_tx.send(
                                        Event::CertifiedTxn(JobResult::CertifiedTxn(
                                            votes.clone(),
                                            threshold_signature,
                                            txn_id.clone(),
                                            farmer_quorum_key.clone(),
                                            farmer_id.clone(),
                                            txn.clone(),
                                            is_txn_valid,
                                        ))
                                        .into(),
                                    );
                                } else {
                                    error!("Quorum signature generation failed");
                                }
                            }
                        } else {
                            error!("Penalize Farmer for wrong votes by sending Wrong Vote event to CR Quorum");
                        }
                    },
                    /// Job `SignConvergenceBlock` signs the  block hash and
                    /// generates a partial signature for the block
                    Job::SignConvergenceBlock(sig_provider, block) => {
                        if let Ok(block_hash_bytes) = hex::decode(block.hash.clone()) {
                            if let Ok(signature) =
                                sig_provider.generate_partial_signature(block_hash_bytes)
                            {
                                if let Ok(dkg_state) = sig_provider.dkg_state.read() {
                                    if let Some(secret_share) = &dkg_state.secret_key_share {
                                        let _ = self.events_tx.send(
                                            Event::ConvergenceBlockPartialSign(
                                                JobResult::ConvergenceBlockPartialSign(
                                                    block.hash,
                                                    secret_share.public_key_share(),
                                                    signature,
                                                )
                                                .into(),
                                            )
                                            .into(),
                                        );
                                    }
                                }
                            }
                        }
                    },
                },
                Err(_) => {},
            }
        }
    }
}
