use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use block::{Block, BlockHash, Certificate, ProposalBlock};
use bulldag::graph::BullDag;
use crossbeam_channel::Sender;
use dashmap::DashMap;
use events::{Event, EventMessage, EventPublisher, JobResult, Vote};
use hbbft::crypto::{PublicKeyShare, SignatureShare};
use primitives::{
    GroupPublicKey,
    HarvesterQuorumThreshold,
    NodeIdx,
    QuorumThreshold,
    RawSignature,
};
use ritelinked::LinkedHashMap;
use signer::signer::{SignatureProvider, Signer};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tracing::error;
use vrrb_core::{
    bloom::Bloom,
    cache::Cache,
    keypair::KeyPair,
    txn::{QuorumCertifiedTxn, TransactionDigest, VoteReceipt},
};

use crate::{farmer_module::PULL_TXN_BATCH_SIZE, scheduler::Job};

/// `CERTIFIED_TXNS_FILTER_SIZE` is a constant that defines the size of the
/// bloom filter used by the `HarvesterModule` to store the certified
/// transactions. In this case, the bloom filter is used to keep track of the
/// transactions that have been certified by the harvester. The size
/// of the bloom filter is set to 500000, which means that it can store up to
/// 500000 elements with a low probability of false positives.
pub const CERTIFIED_TXNS_FILTER_SIZE: usize = 500000;

///  `BLOCK_CERTIFICATES_CACHE_TTL` with a value of `1800000` represents 30
/// mins,i.e caching certificates for a certain period of time(30mins).
pub const BLOCK_CERTIFICATES_CACHE_TTL: u64 = 1800000;

/// Cache Limit for caching last  `BLOCK_CERTIFICATES_CACHE_LIMIT` limit of
/// convergence blocks
pub const BLOCK_CERTIFICATES_CACHE_LIMIT: usize = 5;

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
///  * `vrrbdb_read_handle`: `vrrbdb_read_handle` is a handle to read data from
///    a VRRB  database.
/// This database is used to store and retrieve data related
///  to the blockchain, such as transactions and blocks.
/// * `convergence_block_certificates`: `convergence_block_certificates` is a
///   cache that stores the
/// convergence certificates for blocks. It maps a block hash to a tuple
/// containing the node index, public key share, and raw signature of the
/// certificate. This cache is used to quickly retrieve convergence certificates
/// during block validation and processing.
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
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub convergence_block_certificates:
        Cache<BlockHash, HashSet<(NodeIdx, PublicKeyShare, RawSignature)>>,
    pub harvester_id: NodeIdx,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    status: ActorState,
    _label: ActorLabel,
    id: ActorId,
    broadcast_events_tx: EventPublisher,
    _events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
    quorum_threshold: QuorumThreshold,
    sync_jobs_sender: Sender<Job>,
    _async_jobs_sender: Sender<Job>,
    pub keypair: KeyPair,
}

impl HarvesterModule {
    pub fn new(
        certified_txns_filter: Bloom,
        sig_provider: Option<SignatureProvider>,
        group_public_key: GroupPublicKey,
        events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
        broadcast_events_tx: EventPublisher,
        quorum_threshold: HarvesterQuorumThreshold,
        dag: Arc<RwLock<BullDag<Block, String>>>,
        sync_jobs_sender: Sender<Job>,
        async_jobs_sender: Sender<Job>,
        vrrbdb_read_handle: VrrbDbReadHandle,
        keypair: KeyPair,
        harvester_id: NodeIdx,
    ) -> Self {
        let quorum_certified_txns = Vec::new();

        Self {
            quorum_certified_txns,
            certified_txns_filter,
            sig_provider,
            vrrbdb_read_handle,
            convergence_block_certificates: Cache::new(
                BLOCK_CERTIFICATES_CACHE_LIMIT,
                BLOCK_CERTIFICATES_CACHE_TTL,
            ),
            status: ActorState::Stopped,
            _label: String::from("FarmerHarvester"),
            id: uuid::Uuid::new_v4().to_string(),
            group_public_key,
            broadcast_events_tx,
            _events_rx: events_rx,
            quorum_threshold,
            votes_pool: DashMap::new(),
            dag,
            sync_jobs_sender,
            _async_jobs_sender: async_jobs_sender,
            keypair,
            harvester_id,
        }
    }

    pub fn name(&self) -> String {
        String::from("FarmerHarvester module")
    }

    /// Generate Certificate for convergence block and then broadcast it to the
    /// network
    async fn generate_and_broadcast_certificate(
        &self,
        block_hash: BlockHash,
        certificates_share: &HashSet<(NodeIdx, PublicKeyShare, RawSignature)>,
        sig_provider: &SignatureProvider,
    ) -> Result<(), theater::TheaterError> {
        if certificates_share.len() >= self.quorum_threshold {
            //Generate a new certificate for the block
            let mut sig_shares = BTreeMap::new();
            certificates_share
                .iter()
                .for_each(|(node_idx, _, signature)| {
                    sig_shares.insert(*node_idx, signature.clone());
                });
            if let Ok(certificate) =
                sig_provider.generate_quorum_signature(self.quorum_threshold as u16, sig_shares)
            {
                let certificate = Certificate {
                    signature: hex::encode(certificate),
                    inauguration: None,
                    root_hash: "".to_string(),
                    next_root_hash: "".to_string(),
                    block_hash,
                };
                self.broadcast_events_tx
                    .send(EventMessage::new(
                        None,
                        Event::SendBlockCertificate(certificate),
                    ))
                    .await
                    .map_err(|err| {
                        theater::TheaterError::Other(format!(
                            "failed to send block certificate: {err}"
                        ))
                    })?
            }
        }
        Ok(())
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
            // The above code is handling an event of type `Vote` in a Rust
            // program. It checks the integrity of the vote by
            // verifying that it comes from the actual voter and prevents
            // double voting. It then adds the vote to a pool of votes for the
            // corresponding transaction and farmer quorum key. If
            // the number of votes in the pool reaches the farmer
            // quorum threshold, it sends a job to certify the transaction
            // using the provided signature provider.
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
                                    farmer_quorum_threshold,
                                )));
                            }
                        }
                    } else {
                        self.votes_pool
                            .insert((vote.txn.id(), farmer_quorum_key), vec![vote]);
                    }
                }
            },
            // This certifies txns once vote threshold is reached.
            Event::CertifiedTxn(job_result) => {
                if let JobResult::CertifiedTxn(
                    votes,
                    certificate,
                    txn_id,
                    farmer_quorum_key,
                    farmer_id,
                    txn,
                    is_txn_valid,
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
                        *txn,
                        certificate,
                        is_txn_valid,
                    ));
                    let _ = self
                        .certified_txns_filter
                        .push(&(txn_id, farmer_quorum_key));
                }
            },

            // Mines proposal block after every X seconds.
            Event::MineProposalBlock(ref_hash, round, epoch, claim) => {
                let txns = self.quorum_certified_txns.iter().take(PULL_TXN_BATCH_SIZE);

                //Read updated claims
                let claim_map = self.vrrbdb_read_handle.claim_store_values();
                let claim_list = claim_map
                    .values()
                    .map(|claim| (claim.hash, claim.clone()))
                    .collect();

                let txns_list: LinkedHashMap<TransactionDigest, QuorumCertifiedTxn> = txns
                    .into_iter()
                    .map(|txn| {
                        if let Err(err) = self.certified_txns_filter.push(&txn.txn().id.to_string())
                        {
                            telemetry::error!(
                                "Error pushing txn to certified txns filter: {}",
                                err
                            );
                        }
                        (txn.txn().id(), txn.clone())
                    })
                    .collect();

                let proposal_block = ProposalBlock::build(
                    ref_hash,
                    round,
                    epoch,
                    txns_list,
                    claim_list,
                    claim,
                    self.keypair.get_miner_secret_key(),
                );
                let _ = self
                    .broadcast_events_tx
                    .send(EventMessage::new(
                        None,
                        Event::MinedBlock(Block::Proposal {
                            block: proposal_block,
                        }),
                    ))
                    .await;
            },
            // it sends a job to sign the convergence block using the signature
            // provider
            Event::SignConvergenceBlock(block) => {
                if let Some(sig_provider) = self.sig_provider.clone() {
                    let _ = self
                        .sync_jobs_sender
                        .send(Job::SignConvergenceBlock(sig_provider, block));
                }
            },

            // Process the job result of signing convergence block and adds the
            // partial signature to the cache for certificate generation
            Event::ConvergenceBlockPartialSign(job_result) => {
                if let JobResult::ConvergenceBlockPartialSign(
                    block_hash,
                    public_key_share,
                    partial_signature,
                ) = job_result
                {
                    if let Some(certificates_share) =
                        self.convergence_block_certificates.get(&block_hash)
                    {
                        let mut new_certificate_share = certificates_share.clone();
                        if let Ok(block_hash_bytes) = hex::decode(block_hash.clone()) {
                            if let Ok(signature) =
                                TryInto::<[u8; 96]>::try_into(partial_signature.clone())
                            {
                                if let Ok(signature_share) = SignatureShare::from_bytes(signature) {
                                    if public_key_share.verify(&signature_share, block_hash_bytes) {
                                        new_certificate_share.insert((
                                            self.harvester_id,
                                            public_key_share,
                                            partial_signature.clone(),
                                        ));
                                        self.convergence_block_certificates.push(
                                            block_hash.clone(),
                                            new_certificate_share.clone(),
                                        );
                                        if let Some(sig_provider) = self.sig_provider.as_ref() {
                                            if new_certificate_share.len()
                                                <= sig_provider.quorum_config.upper_bound as usize
                                            {
                                                self
                                                    .broadcast_events_tx
                                                    .send(EventMessage::new(
                                                        None,
                                                        Event::SendPeerConvergenceBlockSign(
                                                            self.harvester_id,
                                                            block_hash.clone(),
                                                            public_key_share.to_bytes().to_vec(),
                                                            partial_signature,
                                                        ),
                                                    ))
                                                    .await.map_err(|err| theater::TheaterError::Other(
                                                        format!("failed to send peer convergence block sign: {err}")
                                                    ))?;

                                                self.generate_and_broadcast_certificate(
                                                    block_hash,
                                                    &new_certificate_share,
                                                    sig_provider,
                                                )
                                                .await?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            Event::PeerConvergenceBlockSign(
                node_idx,
                block_hash,
                public_key_share_bytes,
                partial_signature,
            ) => {
                let mut pb_key_share = None;
                let preliminary_check = TryInto::<[u8; 48]>::try_into(public_key_share_bytes)
                    .and_then(|public_key_share_bytes| {
                        PublicKeyShare::from_bytes(public_key_share_bytes).map_err(|e| {
                            format!("Invalid Public Key, Expected 48byte array: {e}").into_bytes()
                        })
                    })
                    .and_then(|public_key_share| {
                        pb_key_share = Some(public_key_share);
                        TryInto::<[u8; 96]>::try_into(partial_signature.clone())
                            .and_then(|signature_share_bytes| {
                                SignatureShare::from_bytes(signature_share_bytes).map_err(|e| {
                                    format!("Invalid Signature, Expected 96byte array: {e}")
                                        .into_bytes()
                                })
                            })
                            .and_then(|signature_share| {
                                hex::decode(block_hash.clone())
                                    .map_err(|e| {
                                        format!(
                                            "Invalid Hex Representation of Signature Share: {e}",
                                        )
                                        .into_bytes()
                                    })
                                    .and_then(|block_hash_bytes| {
                                        if public_key_share
                                            .verify(&signature_share, block_hash_bytes)
                                        {
                                            Ok(())
                                        } else {
                                            Err("signature verification failed"
                                                .to_string()
                                                .into_bytes())
                                        }
                                    })
                            })
                    });

                if preliminary_check.is_ok() {
                    if let Some(certificates_share) =
                        self.convergence_block_certificates.get(&block_hash)
                    {
                        let mut new_certificate_share = certificates_share.clone();
                        if let Some(pb_key_share) = pb_key_share {
                            new_certificate_share.insert((
                                node_idx,
                                pb_key_share,
                                partial_signature,
                            ));
                            self.convergence_block_certificates
                                .push(block_hash.clone(), new_certificate_share.clone());
                            if let Some(sig_provider) = self.sig_provider.as_ref() {
                                self.generate_and_broadcast_certificate(
                                    block_hash,
                                    &new_certificate_share,
                                    sig_provider,
                                )
                                .await?;
                            }
                        }
                    }
                }
            },
            Event::PrecheckConvergenceBlock(block, last_confirmed_block_header) => {
                let claims = block.claims.clone();
                let txns = block.txns.clone();
                let proposal_block_hashes = block.header.ref_hashes.clone();
                let mut pre_check = true;
                let mut tmp_proposal_blocks = Vec::new();
                if let Ok(dag) = self.dag.read() {
                    for proposal_block_hash in proposal_block_hashes.iter() {
                        if let Some(block) = dag.get_vertex(proposal_block_hash.clone()) {
                            if let Block::Proposal { block } = block.get_data() {
                                tmp_proposal_blocks.push(block.clone());
                            }
                        }
                    }
                    for (ref_hash, claim_hashset) in claims.iter() {
                        match dag.get_vertex(ref_hash.clone()) {
                            Some(block) => {
                                if let Block::Proposal { block } = block.get_data() {
                                    for claim_hash in claim_hashset.iter() {
                                        if !block.claims.contains_key(claim_hash) {
                                            pre_check = false;
                                            break;
                                        }
                                    }
                                }
                            },
                            None => {
                                pre_check = false;
                                break;
                            },
                        }
                    }
                    if pre_check {
                        for (ref_hash, txn_digest_set) in txns.iter() {
                            match dag.get_vertex(ref_hash.clone()) {
                                Some(block) => {
                                    if let Block::Proposal { block } = block.get_data() {
                                        for txn_digest in txn_digest_set.iter() {
                                            if !block.txns.contains_key(txn_digest) {
                                                pre_check = false;
                                                break;
                                            }
                                        }
                                    }
                                },
                                None => {
                                    pre_check = false;
                                    break;
                                },
                            }
                        }
                    }
                }
                if pre_check {
                    self.broadcast_events_tx
                        .send(EventMessage::new(
                            None,
                            Event::CheckConflictResolution((
                                tmp_proposal_blocks,
                                last_confirmed_block_header.round,
                                last_confirmed_block_header.next_block_seed,
                                block,
                            )),
                        ))
                        .await
                        .map_err(|err| {
                            theater::TheaterError::Other(format!(
                                "failed to send conflict resolution check: {err}"
                            ))
                        })?
                }
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
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    use bulldag::graph::BullDag;
    use events::{Event, EventMessage, JobResult, DEFAULT_BUFFER};
    use lazy_static::lazy_static;
    use primitives::Address;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig};
    use theater::{Actor, ActorImpl, ActorState};
    use vrrb_core::{account::Account, bloom::Bloom, keypair::Keypair};

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

        let mut db_config = VrrbDbConfig::default();

        let temp_dir_path = std::env::temp_dir();
        let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

        db_config.with_path(db_path);

        let db = VrrbDb::new(db_config);

        let vrrbdb_read_handle = db.read_handle();

        let harvester_swarm_module = HarvesterModule::new(
            Bloom::new(10000),
            None,
            vec![],
            events_rx,
            broadcast_events_tx,
            2,
            Arc::new(RwLock::new(BullDag::new())),
            sync_jobs_sender,
            async_jobs_sender,
            vrrbdb_read_handle,
            Keypair::random(),
            1u16,
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
