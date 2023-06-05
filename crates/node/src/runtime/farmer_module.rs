use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};

use async_trait::async_trait;
use crossbeam_channel::Sender;
use events::{Event, EventMessage, EventPublisher, JobResult};
use maglev::*;
use mempool::mempool::{LeftRightMempool, TxnStatus};
use primitives::{GroupPublicKey, NodeIdx, PeerId, QuorumThreshold};
use signer::signer::SignatureProvider;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use vrrb_core::txn::{TransactionDigest, Txn};

use crate::scheduler::Job;

pub const PULL_TXN_BATCH_SIZE: usize = 100;

/// The FarmerModule is responsible to validate and vote on transactions within
/// mempool.
///
/// Properties:
///
/// * `tx_mempool`: `tx_mempool` is a `LeftRightMempool` struct that represents
///   the transaction mempool
/// of the `FarmerModule`. It is used to store and manage pending transactions
/// before they are voted.
/// * `group_public_key`: The `group_public_key` property is a distributed group
///   generated public key used
/// for combining votes on threshold.
/// `quorum_threshold` property to determine the minimum number of votes
/// required to certify Txn
/// * `sig_provider`: The `sig_provider` property is an optional
///   `SignatureProvider` that can be used to
/// sign messages or transactions in the `FarmerModule`. It is likely used for
/// cryptographic operations related to the farmer's participation in the
/// network.
/// * `farmer_id`: `farmer_id` is a variable of type `PeerId` which represents
///   the unique identifier of
/// the farmer node in the network. It is likely used to distinguish this node
/// from other nodes in the network and to facilitate communication between
/// nodes.
/// * `farmer_node_idx`: `farmer_node_idx` is a property that represents the
///   index of the farmer node in
/// the network. It is likely used to identify the position of the farmer node
/// in the network topology and to facilitate communication and coordination
/// with other nodes in the network.
/// * `status`: The `status` property is an instance of the `ActorState` enum,
///   which represents the
/// current state of the `FarmerModule` actor. The possible states are defined
/// by the enum variants.
/// * `label`: The `label` property is of type `ActorLabel` and is used to
///   identify the type of actor
/// this struct represents. It is likely an enum that defines different types of
/// actors in the system.
/// * `id`: The `id` property is an `ActorId` which is a unique identifier for
///   the `FarmerModule` actor
/// instance. It is used to distinguish this actor from other actors in the
/// system.
/// * `broadcast_events_tx`: `broadcast_events_tx` is an `UnboundedSender` that
///   is used to send events
/// to other actors in the system. It is unbounded, meaning that it can hold an
/// unlimited number of events until they are consumed by the receiving actors.
/// * `quorum_threshold`: The `quorum_threshold` property is a value that
///   represents the minimum number
/// of nodes required to reach consensus in the network. In other words, if the
/// number of nodes that agree on a particular decision is less than the
/// `quorum_threshold`, the decision is not considered valid. This is often used
/// * `sync_jobs_sender`: `sync_jobs_sender` is a `Sender` object used to send
///   synchronous jobs to the
/// `FarmerModule` actor. It is used to communicate with other actors in the
/// system and coordinate their actions. The `Sender` object is part of Rust's
/// standard library and is used to send messages between
/// * `async_jobs_sender`: `async_jobs_sender` is a property of the
///   `FarmerModule` struct. It is of type
/// `Sender<Job>`, which is a channel sender used to send asynchronous jobs to
/// the module. This property is likely used to handle tasks that can be
/// executed in the background without blocking the main
pub struct FarmerModule {
    pub tx_mempool: LeftRightMempool,
    pub group_public_key: GroupPublicKey,
    pub sig_provider: Option<SignatureProvider>,
    pub farmer_id: PeerId,
    pub farmer_node_idx: NodeIdx,
    pub harvester_peers: HashSet<SocketAddr>,
    pub neighbouring_farmer_quorum_peers: HashMap<GroupPublicKey, HashSet<SocketAddr>>,
    status: ActorState,
    _label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: EventPublisher,
    quorum_threshold: QuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    _async_jobs_sender: Sender<Job>,
}

impl FarmerModule {
    pub fn new(
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        farmer_id: PeerId,
        farmer_node_idx: NodeIdx,
        broadcast_events_tx: EventPublisher,
        quorum_threshold: QuorumThreshold,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
    ) -> Self {
        let lrmpooldb = LeftRightMempool::new();
        Self {
            sig_provider,
            tx_mempool: lrmpooldb,
            status: ActorState::Stopped,
            _label: String::from("Farmer"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            farmer_id,
            farmer_node_idx,
            broadcast_events_tx,
            quorum_threshold,
            sync_jobs_sender,
            _async_jobs_sender: async_jobs_sender,
            harvester_peers: Default::default(),
            neighbouring_farmer_quorum_peers: HashMap::default(),
        }
    }

    pub fn insert_txn(&mut self, txn: Txn) {
        let _ = self.tx_mempool.insert(txn);
    }

    /// This function updates the status of a transaction in a transaction
    /// mempool.
    ///
    /// Arguments:
    ///
    /// * `txn_id`: The `txn_id` parameter is of type `TransactionDigest` and
    ///   represents the unique
    /// identifier of a transaction.
    /// * `status`: The `status` parameter is of type `TxnStatus`, which is an
    ///   enum representing the
    /// status of a transaction.
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
impl Handler<EventMessage> for FarmerModule {
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
            Event::AddHarvesterPeer(peer) => {
                self.harvester_peers.insert(peer);
            },
            Event::RemoveHarvesterPeer(peer) => {
                self.harvester_peers.remove(&peer);
            },
            /*
            Event::SyncNeighbouringFarmerQuorum(peers_details) => {
                for (group_public_key, addressess) in peers_details {
                    self.neighbouring_farmer_quorum_peers
                        .insert(group_public_key, addressess);
                }
            },*/
            //Event  "Farm" fetches a batch of transactions from a transaction mempool and sends
            // them to scheduler to get it validated and voted
            Event::Farm => {
                let txns = self.tx_mempool.fetch_txns(PULL_TXN_BATCH_SIZE);
                let keys: Vec<GroupPublicKey> = self
                    .neighbouring_farmer_quorum_peers
                    .keys()
                    .cloned()
                    .collect();
                let maglev_hash_ring = Maglev::new(keys);
                let mut new_txns = vec![];
                for txn in txns.into_iter() {
                    if let Some(group_public_key) = maglev_hash_ring.get(&txn.0.clone()).cloned() {
                        if group_public_key == self.group_public_key {
                            new_txns.push(txn);
                        } else if let Some(broadcast_addresses) =
                            self.neighbouring_farmer_quorum_peers.get(&group_public_key)
                        {
                            let addresses: Vec<SocketAddr> =
                                broadcast_addresses.iter().cloned().collect();
                            if let Err(err) = self
                                .broadcast_events_tx
                                .send(EventMessage::new(
                                    None,
                                    Event::ForwardTxn((txn.1.clone(), addresses.clone())),
                                ))
                                .await
                            {
                                let err_msg = format!(
                                    "failed to forward txn {:?} to peers {addresses:?}: {err}",
                                    txn.1
                                );
                                return Err(theater::TheaterError::Other(err_msg));
                            }
                        }
                    } else {
                        new_txns.push(txn);
                    }
                }

                if let Some(sig_provider) = self.sig_provider.clone() {
                    if let Err(err) = self.sync_jobs_sender.send(Job::Farm((
                        new_txns,
                        self.farmer_id.clone(),
                        self.farmer_node_idx,
                        self.group_public_key.clone(),
                        sig_provider,
                        self.quorum_threshold,
                    ))) {
                        telemetry::error!("error sending job to scheduler: {}", err);
                    }
                }
            },
            // Receive the Vote from scheduler
            Event::ProcessedVotes(JobResult::Votes((votes, farmer_quorum_threshold))) => {
                for vote in votes.iter().flatten() {
                    if let Err(err) = self
                        .broadcast_events_tx
                        .send(Event::Vote(vote.clone(), farmer_quorum_threshold).into())
                        .await
                    {
                        let err_msg = format!("failed to send vote: {err}");
                        return Err(theater::TheaterError::Other(err_msg));
                    }
                }
            },
            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
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
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use dkg_engine::{test_utils, types::config::ThresholdConfig};
    use events::{Event, EventMessage, JobResult, DEFAULT_BUFFER};
    use lazy_static::lazy_static;
    use primitives::Address;
    use secp256k1::Message;
    use signer::signer::SignatureProvider;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
    use theater::{Actor, ActorImpl, ActorState};
    use validator::validator_core_manager::ValidatorCoreManager;
    use vrrb_core::{
        account::Account,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::{
        farmer_module::FarmerModule,
        scheduler::{Job, JobSchedulerController},
    };

    #[tokio::test]
    async fn farmer_module_starts_and_stops() {
        let (broadcast_events_tx, _) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);
        let (_, _clear_filter_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
        let (sync_jobs_sender, _sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, _async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

        let (_sync_jobs_status_sender, _sync_jobs_status_receiver) =
            crossbeam_channel::unbounded::<JobResult>();
        let (_async_jobs_status_sender, _async_jobs_status_receiver) =
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
        );
        let mut farmer_swarm_module = ActorImpl::new(farmer_module);

        let (ctrl_tx, mut ctrl_rx) =
            tokio::sync::broadcast::channel::<EventMessage>(DEFAULT_BUFFER);

        assert_eq!(farmer_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            farmer_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(farmer_swarm_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }
    lazy_static! {
        static ref STATE_SNAPSHOT: HashMap<Address, Account> = HashMap::new();
    }

    #[tokio::test]
    async fn farmer_farm_cast_vote() {
        let (events_tx, _) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);

        let (broadcast_events_tx, _broadcast_events_rx) =
            tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);

        let (_, _clear_filter_rx) = tokio::sync::mpsc::channel::<EventMessage>(DEFAULT_BUFFER);

        let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
        let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

        let mut db_config = VrrbDbConfig::default();
        let temp_dir_path = std::env::temp_dir();
        let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());
        db_config.with_path(db_path);

        let db = VrrbDb::new(db_config);
        let vrrbdb_read_handle = db.read_handle();

        let mut job_scheduler = JobSchedulerController::new(
            vec![0],
            events_tx,
            sync_jobs_receiver,
            async_jobs_receiver,
            ValidatorCoreManager::new(8).unwrap(),
            vrrbdb_read_handle,
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
        );
        let keypair = KeyPair::random();
        let recv_kp = KeyPair::random();
        let mut txns = HashSet::<Txn>::new();

        let _now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // let txn_id = String::from("1");
        let sender_address = Address::new(*keypair.get_miner_public_key());
        let receiver_address = Address::new(*recv_kp.get_miner_public_key());
        let txn_amount: u128 = 1010101;

        for n in 1..101 {
            let sig = keypair.miner_kp.0.sign_ecdsa(Message::from_hashed_data::<
                secp256k1::hashes::sha256::Hash,
            >(
                b"
    vrrb",
            ));

            let txn = Txn::new(NewTxnArgs {
                timestamp: 0,
                sender_address: sender_address.clone(),
                sender_public_key: *keypair.get_miner_public_key(),
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
        let (ctrl_tx, mut ctrl_rx) = tokio::sync::broadcast::channel::<EventMessage>(10000);
        assert_eq!(farmer_swarm_module.status(), ActorState::Stopped);

        let handle = tokio::spawn(async move {
            farmer_swarm_module.start(&mut ctrl_rx).await.unwrap();
            assert_eq!(farmer_swarm_module.status(), ActorState::Terminating);
        });

        ctrl_tx.send(Event::Farm.into()).unwrap();
        ctrl_tx.send(Event::Stop.into()).unwrap();
        handle.await.unwrap();
    }
}
