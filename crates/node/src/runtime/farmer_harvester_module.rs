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
    QuorumType,
    RawSignature,
};
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use signer::signer::{SignatureProvider, Signer};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::{broadcast::error::TryRecvError, mpsc::UnboundedSender};
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

/// `FarmerHarvesterModule` is a struct that contains a bunch of `Option`s, a
/// `Bloom` filter, a `GroupPublicKey`, a `SignatureProvider`, a `PeerId`, a
/// `NodeIdx`, an `ActorState`, an `ActorLabel`, an `ActorId`, a
/// `tokio::sync::mpsc::UnboundedSender`, a
/// `tokio::sync::mpsc::UnboundedReceiver`, a `FarmerQuorumThreshold`, a
/// `HarvesterQuorumThreshold`, a
///
/// Properties:
///
/// * `quorum_certified_txns`: This is a list of transactions that have been
///   certified by the farmer.
/// * `certified_txns_filter`: Bloom - A bloom filter that contains all the
///   transactions that have been
/// certified by the farmer.
/// * `quorum_type`: The type of quorum that the farmer is currently using.
/// * `tx_mempool`: This is the mempool that the farmer uses to store
///   transactions.
/// * `votes_pool`: This is a map of (TxHashString, FarmerId) to a vector of
///   votes.
/// * `group_public_key`: The public key of the group that the farmer is a
///   member of.
/// * `sig_provider`: This is the signature provider that the farmer will use to
///   sign the transactions.
/// * `farmer_id`: PeerId - The PeerId of the farmer.
/// * `farmer_node_idx`: NodeIdx - The index of the node that this farmer is
///   running on.
/// * `status`: ActorState - The state of the actor.
/// * `label`: ActorLabel - The label of the actor.
/// * `id`: ActorId - The id of the actor.
/// * `broadcast_events_tx`: This is the channel that the FarmerHarvesterModule
///   uses to send events to
/// the FarmerHarvesterActor.
/// * `clear_filter_rx`: This is a channel that the farmer listens on for a
///   message to clear the bloom
/// filter.
/// * `farmer_quorum_threshold`: The minimum number of farmers that must sign a
///   transaction before it
/// can be harvested.
/// * `harvester_quorum_threshold`: The minimum number of votes required to
///   certify a transaction.
/// * `sync_jobs_sender`: Sender<Job>
/// * `async_jobs_sender`: Sender<Job> - This is a channel that the
///   FarmerHarvesterModule uses to send
/// jobs to the async_jobs_worker.
/// * `sync_jobs_status_receiver`: Receiver<JobResult>
/// * `async_jobs_status_receiver`: Receiver<JobResult>
pub struct FarmerHarvesterModule {
    pub quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    pub certified_txns_filter: Bloom,
    pub quorum_type: Option<QuorumType>,
    pub tx_mempool: Option<LeftRightMempool>,
    pub votes_pool: DashMap<(TransactionDigest, String), Vec<Vote>>,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    pub farmer_id: PeerId,
    pub farmer_node_idx: NodeIdx,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
    clear_filter_rx: tokio::sync::mpsc::UnboundedReceiver<DirectedEvent>,
    farmer_quorum_threshold: FarmerQuorumThreshold,
    harvester_quorum_threshold: HarvesterQuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
    sync_jobs_status_receiver: Receiver<JobResult>,
    async_jobs_status_receiver: Receiver<JobResult>,
}


impl FarmerHarvesterModule {
    pub fn new(
        certified_txns_filter: Bloom,
        quorum_type: Option<QuorumType>,
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        farmer_id: PeerId,
        farmer_node_idx: NodeIdx,
        broadcast_events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
        clear_filter_rx: tokio::sync::mpsc::UnboundedReceiver<DirectedEvent>,
        farmer_quorum_threshold: FarmerQuorumThreshold,
        harvester_quorum_threshold: HarvesterQuorumThreshold,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
        sync_jobs_status_receiver: Receiver<JobResult>,
        async_jobs_status_receiver: Receiver<JobResult>,
    ) -> Self {
        let lrmpooldb = if let Some(QuorumType::Farmer) = quorum_type {
            Some(LeftRightMempool::new())
        } else {
            None
        };
        let quorum_certified_txns = Vec::new();
        let farmer_harvester = Self {
            quorum_certified_txns,
            certified_txns_filter,
            quorum_type,
            sig_provider,
            tx_mempool: lrmpooldb,
            status: ActorState::Stopped,
            label: String::from("FarmerHarvester"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            farmer_id,
            farmer_node_idx,
            broadcast_events_tx: broadcast_events_tx.clone(),
            clear_filter_rx,
            farmer_quorum_threshold,
            harvester_quorum_threshold,
            votes_pool: DashMap::new(),
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver: sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver: async_jobs_status_receiver.clone(),
        };
        farmer_harvester
    }

    fn process_sync_job_status(
        &mut self,
        broadcast_events_tx: UnboundedSender<DirectedEvent>,
        sync_jobs_status_receiver: Receiver<JobResult>,
    ) {
        loop {
            let job_result = sync_jobs_status_receiver.recv().unwrap();
            match job_result {
                JobResult::Votes((votes, farmer_quorum_threshold)) => {
                    for vote_opt in votes.iter() {
                        if let Some(vote) = vote_opt {
                            let _ = broadcast_events_tx.send((
                                Topic::Network,
                                Event::Vote(
                                    vote.clone(),
                                    QuorumType::Harvester,
                                    farmer_quorum_threshold,
                                ),
                            ));
                        }
                    }
                },
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
impl Handler<Event> for FarmerHarvesterModule {
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
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::Farm => {
                if let Some(QuorumType::Farmer) = self.quorum_type {
                    if let Some(tx_mempool) = self.tx_mempool.borrow_mut() {
                        let txns = tx_mempool.fetch_txns(PULL_TXN_BATCH_SIZE);
                        if let Some(sig_provider) = self.sig_provider.clone() {
                            let _ = self.sync_jobs_sender.send(Job::Farm((
                                txns,
                                self.farmer_id.clone(),
                                self.farmer_node_idx,
                                self.group_public_key.clone(),
                                sig_provider.clone(),
                                self.farmer_quorum_threshold,
                            )));
                        }
                    }
                }
            },
            Event::Vote(vote, quorum, farmer_quorum_threshold) => {
                if let QuorumType::Farmer = quorum {
                    error!("Farmer cannot process votes ");
                } else if let QuorumType::Harvester = quorum {
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
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

pub trait QuorumMember {}
// TODO: Move this to primitives
pub type QuorumId = String;
pub type QuorumPubkey = String;

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
    async fn farmer_harvester_runtime_module_starts_and_stops() {
        let (broadcast_events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

        let (sync_jobs_status_sender, sync_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let (async_jobs_status_sender, async_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let farmer_harvester_swarm_module = FarmerHarvesterModule::new(
            Bloom::new(10000),
            None,
            None,
            vec![],
            vec![],
            0,
            broadcast_events_tx,
            clear_filter_rx,
            2,
            2,
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver.clone(),
        );
        let mut farmer_harvester_swarm_module = ActorImpl::new(farmer_harvester_swarm_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(farmer_harvester_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            farmer_harvester_swarm_module
                .start(&mut ctrl_rx)
                .await
                .unwrap();
            assert_eq!(
                farmer_harvester_swarm_module.status(),
                ActorState::Terminating
            );
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }
    lazy_static! {
        static ref STATE_SNAPSHOT: StateSnapshot = StateSnapshot {
            accounts: HashMap::new(),
        };
    }

    #[tokio::test]
    async fn farmer_harvester_farm_cast_vote() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (broadcast_events_tx, broadcast_events_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (sync_jobs_status_sender, sync_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let (async_jobs_status_sender, async_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();

        let mut job_scheduler = JobSchedulerController::new(
            vec![0],
            sync_jobs_receiver,
            async_jobs_receiver,
            sync_jobs_status_sender,
            async_jobs_status_sender,
            ValidatorCoreManager::new(TxnValidator::new(), 8).unwrap(),
            &*STATE_SNAPSHOT,
        );
        thread::spawn(move || {
            job_scheduler.execute_sync_jobs();
        });
        let mut dkg_engines = test_utils::generate_dkg_engine_with_states().await;
        let dkg_engine = dkg_engines.pop().unwrap();
        let group_public_key = dkg_engine
            .dkg_state
            .public_key_set
            .clone()
            .unwrap()
            .public_key()
            .to_bytes()
            .to_vec();
        let sig_provider = SignatureProvider {
            dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine.dkg_state)),
            quorum_config: ThresholdConfig {
                threshold: 2,
                upper_bound: 4,
            },
        };
        let mut farmer_harvester_swarm_module = FarmerHarvesterModule::new(
            Bloom::new(10000),
            Some(Farmer),
            Some(sig_provider),
            group_public_key,
            dkg_engine.secret_key.public_key().to_bytes().to_vec(),
            1,
            broadcast_events_tx,
            clear_filter_rx,
            2,
            2,
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver.clone(),
        );
        let keypair = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // let txn_id = String::from("1");
        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 1..101 {
            let sig = keypair
                .miner_kp
                .0
                .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb"));

            let txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
                signature: sig,
            });
            txns.insert(txn);
        }
        if let Some(tx_mempool) = farmer_harvester_swarm_module.tx_mempool.borrow_mut() {
            let _ = tx_mempool.extend(txns);
        }
        let mut farmer_harvester_swarm_module = ActorImpl::new(farmer_harvester_swarm_module);
        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10000);
        assert_eq!(farmer_harvester_swarm_module.status(), ActorState::Stopped);
        let handle = tokio::spawn(async move {
            farmer_harvester_swarm_module
                .start(&mut ctrl_rx)
                .await
                .unwrap();
            assert_eq!(
                farmer_harvester_swarm_module.status(),
                ActorState::Terminating
            );
        });
        ctrl_tx.send(Event::Farm.into()).unwrap();
        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();

        let job_status = sync_jobs_status_receiver.recv().unwrap();
        is_enum_variant!(job_status, JobResult::Votes { .. });
    }

    #[tokio::test]
    async fn farmer_harvester_harvest_votes() {
        let (events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let mut dkg_engines = test_utils::generate_dkg_engine_with_states().await;
        let mut farmers = vec![];
        let mut broadcast_rxs = vec![];
        let mut sync_job_status_receivers = vec![];
        let mut tmp = vec![];
        while dkg_engines.len() > 0 {
            let (broadcast_events_tx, mut broadcast_events_rx) =
                tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
            let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
            let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
            let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
            let (sync_jobs_status_sender, sync_jobs_status_receiver) =
                crossbeam_channel::unbounded::<JobResult>();
            let (async_jobs_status_sender, async_jobs_status_receiver) =
                crossbeam_channel::unbounded::<JobResult>();

            let mut job_scheduler = JobSchedulerController::new(
                vec![0],
                sync_jobs_receiver,
                async_jobs_receiver,
                sync_jobs_status_sender,
                async_jobs_status_sender,
                ValidatorCoreManager::new(TxnValidator::new(), 8).unwrap(),
                &*STATE_SNAPSHOT,
            );
            thread::spawn(move || {
                job_scheduler.execute_sync_jobs();
            });

            let (broadcast_events_tx, mut broadcast_events_rx) =
                tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
            broadcast_rxs.push(broadcast_events_rx);
            let dkg_engine = dkg_engines.pop().unwrap();
            let group_public_key = dkg_engine
                .dkg_state
                .public_key_set
                .clone()
                .unwrap()
                .public_key()
                .to_bytes()
                .to_vec();
            let sig_provider = SignatureProvider {
                dkg_state: std::sync::Arc::new(std::sync::RwLock::new(dkg_engine.dkg_state)),
                quorum_config: ThresholdConfig {
                    threshold: 2,
                    upper_bound: 4,
                },
            };
            tmp.push((
                sig_provider.clone(),
                dkg_engine
                    .secret_key
                    .public_key()
                    .to_bytes()
                    .to_vec()
                    .clone(),
                dkg_engine.node_idx,
                group_public_key.clone(),
            ));
            let mut farmer = FarmerHarvesterModule::new(
                Bloom::new(10000),
                Some(Farmer),
                Some(sig_provider),
                group_public_key,
                dkg_engine.secret_key.public_key().to_bytes().to_vec(),
                dkg_engine.node_idx,
                broadcast_events_tx.clone(),
                clear_filter_rx,
                2,
                2,
                sync_jobs_sender,
                async_jobs_sender,
                sync_jobs_status_receiver.clone(),
                async_jobs_status_receiver,
            );
            farmer.quorum_certified_txns = Vec::<QuorumCertifiedTxn>::new();
            sync_job_status_receivers.push(sync_jobs_status_receiver);
            farmers.push(farmer);
        }

        let keypair = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let sender_address = String::from("aaa1");
        let receiver_address = String::from("bbb1");
        let txn_amount: u128 = 1010101;

        for n in 0..1 {
            let sig = keypair
                .miner_kp
                .0
                .sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb"));

            let mut txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("aaa1"),
                sender_public_key: keypair.get_miner_public_key().clone(),
                receiver_address: receiver_address.clone(),
                token: None,
                amount: txn_amount + n,
                signature: sig,
                validators: Some(HashMap::<String, bool>::new()),
                nonce: 0,
            });

            txns.insert(txn);
        }

        let mut ctrx_txns = vec![];
        let mut handles = vec![];
        let mut sync_status_receivers = vec![];
        for mut farmer in farmers.into_iter() {
            let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

            if let Some(tx_mempool) = farmer.tx_mempool.borrow_mut() {
                let _ = tx_mempool.extend(txns.clone());
            }
            sync_status_receivers.push(farmer.sync_jobs_status_receiver.clone());
            let mut farmer_harvester_swarm_module = ActorImpl::new(farmer);

            let handle = tokio::spawn(async move {
                farmer_harvester_swarm_module
                    .start(&mut ctrl_rx)
                    .await
                    .unwrap();
                assert_eq!(
                    farmer_harvester_swarm_module.status(),
                    ActorState::Terminating
                );
            });
            ctrx_txns.push(ctrl_tx);
            handles.push(handle);
        }

        ctrx_txns.get(0).unwrap().send(Event::Farm.into()).unwrap();

        ctrx_txns.get(0).unwrap().send(Event::Stop.into()).unwrap();
        handles.get_mut(0).unwrap().await.unwrap();
        let receiver = sync_status_receivers.get(0).unwrap();
        let mut ballot = vec![];

        let status = receiver.recv();
        match status {
            Ok(job_status) => {
                if let JobResult::Votes((votes, threshold)) = job_status {
                    let vote = votes.get(0).unwrap().as_ref().unwrap().clone();
                    ballot.push(vote.clone());
                    for ((sig_provider, farmer_id, farmer_node_idx, group_public_key)) in
                        tmp.into_iter()
                    {
                        if farmer_node_idx == 0 {
                            continue;
                        }
                        let txn = vote.txn.clone();
                        let txn_bytes = bincode::serialize(&txn).unwrap();
                        let signature = sig_provider.generate_partial_signature(txn_bytes).unwrap();
                        let new_vote = Vote {
                            farmer_id,
                            farmer_node_id: farmer_node_idx,
                            signature,
                            txn,
                            quorum_public_key: group_public_key,
                            quorum_threshold: threshold,
                            execution_result: None,
                        };
                        ballot.push(new_vote);
                    }
                }
            },
            Err(e) => {},
        }

        for vote in ballot.iter() {
            ctrx_txns
                .get(1)
                .unwrap()
                .send(Event::Vote(vote.clone(), QuorumType::Harvester, 2))
                .unwrap();
        }
        ctrx_txns.get(1).unwrap().send(Event::Stop.into()).unwrap();
        handles.get_mut(1).unwrap().await.unwrap();

        let harvester_receiver = sync_status_receivers.get(1).unwrap();
        let certified_txn = harvester_receiver.recv().unwrap();
        is_enum_variant!(certified_txn, JobResult::CertifiedTxn { .. });
        ctrx_txns.get(2).unwrap().send(Event::Stop.into()).unwrap();
        handles.get_mut(2).unwrap().await.unwrap();
        ctrx_txns.get(3).unwrap().send(Event::Stop.into()).unwrap();
        handles.get_mut(3).unwrap().await.unwrap();
    }
}
