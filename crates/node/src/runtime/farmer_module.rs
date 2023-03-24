use std::thread;
use async_trait::async_trait;
use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;
use events::{DirectedEvent, Event, QuorumCertifiedTxn, Topic, Vote, VoteReceipt};
use lr_trie::ReadHandleFactory;
use mempool::mempool::{LeftRightMempool, TxnStatus};
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::{GroupPublicKey, NodeIdx, PeerId, QuorumThreshold, QuorumType, RawSignature};
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
    result::Result,
    scheduler::{Job, JobResult},
    NodeError,
};

pub const PULL_TXN_BATCH_SIZE: usize = 1000;

/// `FarmerModule` is responsible for voting on transactions present in mempool
///
/// Properties:
///
/// * `tx_mempool`: This is the mempool that the farmer will use to store
///   transactions.
/// * `group_public_key`: The public key of the group that the farmer is a
///   member of.
/// * `sig_provider`: This is the signature provider that will be used to sign
///   the transactions.
/// * `farmer_id`: PeerId - The peer id of the farmer.
/// * `farmer_node_idx`: NodeIdx - The index of the node in the network.
/// * `status`: The current state of the actor.
/// * `label`: The label of the actor.
/// * `id`: ActorId - The unique identifier of the actor.
/// * `broadcast_events_tx`: This is the channel that the farmer uses to send
///   events to the network.
/// * `quorum_threshold`: QuorumThreshold,
/// * `sync_jobs_sender`: Sender<Job>
/// * `async_jobs_sender`: Sender<Job>
/// * `sync_jobs_status_receiver`: Receiver<JobResult>
/// * `async_jobs_status_receiver`: Receiver<JobResult>
pub struct FarmerModule {
    pub tx_mempool: LeftRightMempool,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    pub farmer_id: PeerId,
    pub farmer_node_idx: NodeIdx,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: UnboundedSender<DirectedEvent>,
    quorum_threshold: QuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
    sync_jobs_status_receiver: Receiver<JobResult>,
    async_jobs_status_receiver: Receiver<JobResult>,
}

impl FarmerModule {
    pub fn new(
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        farmer_id: PeerId,
        farmer_node_idx: NodeIdx,
        broadcast_events_tx: UnboundedSender<DirectedEvent>,
        quorum_threshold: QuorumThreshold,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
        sync_jobs_status_receiver: Receiver<JobResult>,
        async_jobs_status_receiver: Receiver<JobResult>,
    ) -> Self {
        let lrmpooldb = LeftRightMempool::new();
        let farmer = Self {
            sig_provider,
            tx_mempool: lrmpooldb,
            status: ActorState::Stopped,
            label: String::from("Farmer"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            farmer_id,
            farmer_node_idx,
            broadcast_events_tx: broadcast_events_tx.clone(),
            quorum_threshold,
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver: sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver: async_jobs_status_receiver.clone(),
        };
        farmer
    }

    /// > This function receives the results of the jobs that the farmer has
    /// > completed and broadcasts
    /// the votes to the harvester network
    ///
    /// Arguments:
    ///
    /// * `broadcast_events_tx`: This is the channel that the farmer will use to
    ///   send events to the rest
    /// of the network.
    /// * `sync_jobs_status_receiver`: This is a channel that receives the
    ///   results of the sync jobs.
    fn process_sync_job_status(
        &mut self,
        broadcast_events_tx: UnboundedSender<DirectedEvent>,
        sync_jobs_status_receiver: Receiver<JobResult>,
    ) {
        loop {
            let job_result = sync_jobs_status_receiver.recv().unwrap();
            match job_result {
                JobResult::Votes((votes, farmer)) => {
                    for vote_opt in votes.iter() {
                        if let Some(vote) = vote_opt {
                            let _ = broadcast_events_tx.send(
                                Event::Vote(
                                    vote.clone(), 
                                    QuorumType::Harvester, 
                                    farmer
                                )
                            );
                        }
                    }
                },
                _ => {
                    error!("Farmers can only vote on Transactions.")
                },
            }
        }
    }

    pub fn insert_txn(&mut self, txn: Txn) {
        let _ = self.tx_mempool.insert(txn);
    }

    pub fn update_txn_status(&mut self, txn_id: TransactionDigest, status: TxnStatus) {
        let txn_record_opt = self.tx_mempool.get(&txn_id);
        if let Some(mut txn_record) = txn_record_opt {
            txn_record.status = status;
            self.remove_txn(txn_id);
            self.insert_txn(txn_record.txn);
        }
    }

    pub fn remove_txn(&mut self, txn_id: TransactionDigest) {
        let _ = self.tx_mempool.remove(&txn_id);
    }

    pub fn name(&self) -> String {
        String::from("FarmerHarvester module")
    }
}

#[async_trait]
impl Handler<Event> for FarmerModule {
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
                let txns = self.tx_mempool.fetch_txns(PULL_TXN_BATCH_SIZE);
                if let Some(sig_provider) = self.sig_provider.clone() {
                    let _ = self.sync_jobs_sender.send(Job::Farm((
                        txns,
                        self.farmer_id.clone(),
                        self.farmer_node_idx,
                        self.group_public_key.clone(),
                        sig_provider.clone(),
                        self.quorum_threshold,
                    )));
                }
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
    async fn farmer_module_starts_and_stops() {
        let (broadcast_events_tx, _) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (_, clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<DirectedEvent>();
        let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

        let (sync_jobs_status_sender, sync_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let (async_jobs_status_sender, async_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let farmer_module = FarmerModule::new(
            None,
            vec![],
            vec![],
            0,
            broadcast_events_tx,
            2,
            sync_jobs_sender,
            async_jobs_sender,
            sync_jobs_status_receiver.clone(),
            async_jobs_status_receiver.clone(),
        );
        let mut farmer_swarm_module = ActorImpl::new(farmer_module);

        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10);

        assert_eq!(farmer_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            farmer_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(farmer_swarm_module.status(), ActorState::Terminating);
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
    async fn farmer_farm_cast_vote() {
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
        let mut farmer = FarmerModule::new(
            Some(sig_provider),
            group_public_key,
            dkg_engine.secret_key.public_key().to_bytes().to_vec(),
            1,
            broadcast_events_tx,
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

        let _ = farmer.tx_mempool.extend(txns);

        let mut farmer_swarm_module = ActorImpl::new(farmer);
        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<Event>(10000);
        assert_eq!(farmer_swarm_module.status(), ActorState::Stopped);
        let handle = tokio::spawn(async move {
            farmer_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(farmer_swarm_module.status(), ActorState::Terminating);
        });
        ctrl_tx.send(Event::Farm.into()).unwrap();
        ctrl_tx.send(Event::Stop.into()).unwrap();

        handle.await.unwrap();

        let job_status = sync_jobs_status_receiver.recv().unwrap();
        is_enum_variant!(job_status, JobResult::Votes { .. });
    }
}
