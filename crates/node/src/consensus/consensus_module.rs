use std::collections::HashSet;

use block::{header::BlockHeader, Block, BlockHash, ConvergenceBlock, ProposalBlock, RefHash};
use chrono::Duration;
use dkg_engine::{
    dkg::DkgGenerator,
    prelude::{DkgEngine, DkgEngineConfig},
};
use events::{
    AssignedQuorumMembership, Event, EventMessage, EventPublisher, EventSubscriber, PeerData,
    SyncPeerData, Vote,
};
use hbbft::sync_key_gen::Part;
use laminar::{Packet, SocketEvent};
use maglev::Maglev;
use mempool::{TxnRecord, TxnStatus};
use primitives::{
    Epoch, FarmerQuorumThreshold, GroupPublicKey, NodeId, NodeIdx, NodeType, NodeTypeBytes,
    PKShareBytes, PayloadBytes, ProgramExecutionOutput, PublicKeyShareVec, QuorumPublicKey,
    RawSignature, Round, TxnValidationStatus, ValidatorPublicKey, ValidatorPublicKeyShare,
    ValidatorSecretKey,
};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use signer::signer::SignatureProvider;
use telemetry::error;
use theater::{Actor, ActorId, ActorState, TheaterError};
use vrrb_config::{NodeConfig, QuorumMember, QuorumMembershipConfig};
use vrrb_core::{
    bloom::Bloom,
    claim::Claim,
    keypair::Keypair,
    txn::{QuorumCertifiedTxn, TransactionDigest, Txn},
};

use crate::{state_reader::StateReader, NodeError, Result};

use super::{QuorumModule, QuorumModuleConfig};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

// TODO: Move this to primitives
pub type QuorumId = String;
pub type QuorumPubkey = String;

#[derive(Debug)]
pub struct ConsensusModuleConfig<S: StateReader + Send + Sync> {
    pub events_tx: EventPublisher,
    pub keypair: Keypair,
    pub vrrbdb_read_handle: S,
    pub node_config: NodeConfig,
    pub dkg_generator: DkgEngine,
    pub validator_public_key: ValidatorPublicKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Data {
    Request(RendezvousRequest),
    Response(RendezvousResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendezvousRequest {
    Ping,
    Peers(Vec<u8>),
    Namespace(NodeTypeBytes, QuorumPublicKey),
    RegisterPeer(
        QuorumPublicKey,
        NodeTypeBytes,
        PKShareBytes,
        RawSignature,
        PayloadBytes,
        SyncPeerData,
    ),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RendezvousResponse {
    Pong,
    RequestPeers(QuorumPublicKey),
    Peers(Vec<SyncPeerData>),
    PeerRegistered,
    NamespaceRegistered,
}

#[derive(Debug)]
pub struct ConsensusModule<S: StateReader + Sync + Send + Clone> {
    pub(crate) id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) vrrbdb_read_handle: S,
    pub(crate) quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    pub(crate) keypair: Keypair,
    pub(crate) certified_txns_filter: Bloom,
    pub(crate) quorum_driver: QuorumModule<S>,
    pub(crate) dkg_engine: DkgEngine,
    pub(crate) node_config: NodeConfig,
    //
    // votes_pool: DashMap<(TransactionDigest, String), Vec<Vote>>,
    // group_public_key: GroupPublicKey,
    // sig_provider: Option<SignatureProvider>,
    // vrrbdb_read_handle: VrrbDbReadHandle,
    // convergence_block_certificates:
    //     Cache<BlockHash, HashSet<(NodeIdx, PublicKeyShare, RawSignature)>>,
    //
    // harvester_id: NodeIdx,
    // dag: Arc<RwLock<BullDag<Block, String>>>,
    // quorum_threshold: QuorumThreshold,
    //
    // sync_jobs_sender: Sender<Job>,
    // status: ActorState,
    // id: ActorId,
    // broadcast_events_tx: EventPublisher,
    // _label: ActorLabel,
    // _events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
    // _async_jobs_sender: Sender<Job>,

    //
    // NOTE: harvester types
    //
    // pub quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    // pub certified_txns_filter: Bloom,
    // pub votes_pool: DashMap<(TransactionDigest, String), Vec<Vote>>,
    // pub group_public_key: GroupPublicKey,
    // pub sig_provider: Option<SignatureProvider>,
    // pub vrrbdb_read_handle: VrrbDbReadHandle,
    // pub convergence_block_certificates:
    //     Cache<BlockHash, HashSet<(NodeIdx, PublicKeyShare, RawSignature)>>,
    // pub harvester_id: NodeIdx,
    // pub dag: Arc<RwLock<BullDag<Block, String>>>,
    // status: ActorState,
    // _label: ActorLabel,
    // id: ActorId,
    // broadcast_events_tx: EventPublisher,
    // _events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
    // quorum_threshold: QuorumThreshold,
    // sync_jobs_sender: Sender<Job>,
    // _async_jobs_sender: Sender<Job>,
    // pub keypair: KeyPair,
}

impl<S: StateReader + Send + Sync + Clone> ConsensusModule<S> {
    pub fn new(cfg: ConsensusModuleConfig<S>) -> Self {
        let quorum_module_config = QuorumModuleConfig {
            events_tx: cfg.events_tx.clone(),
            vrrbdb_read_handle: cfg.vrrbdb_read_handle.clone(),
            membership_config: None,
            node_config: cfg.node_config.clone(),
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ActorState::Stopped,
            events_tx: cfg.events_tx,
            vrrbdb_read_handle: cfg.vrrbdb_read_handle,
            quorum_certified_txns: vec![],
            keypair: cfg.keypair,
            certified_txns_filter: Bloom::new(10),
            quorum_driver: QuorumModule::new(quorum_module_config),
            dkg_engine: cfg.dkg_generator,
            node_config: cfg.node_config,
        }
    }

    pub fn validator_public_key_owned(&self) -> ValidatorPublicKey {
        self.keypair.validator_public_key_owned()
    }

    async fn certify_block(&self) {
        //
    }

    async fn mine_genesis_block(&self) {
        //
    }

    async fn mine_proposal_block(
        &mut self,
        ref_hash: RefHash,
        round: Round,
        epoch: Epoch,
        claim: Claim,
    ) -> ProposalBlock {
        let txns = self.quorum_certified_txns.iter().take(PULL_TXN_BATCH_SIZE);

        // NOTE: Read updated claims
        let claim_map = self.vrrbdb_read_handle.claim_store_values();
        let claim_list = claim_map
            .values()
            .map(|claim| (claim.hash, claim.clone()))
            .collect();

        let txns_list: LinkedHashMap<TransactionDigest, QuorumCertifiedTxn> = txns
            .into_iter()
            .map(|txn| {
                if let Err(err) = self.certified_txns_filter.push(&txn.txn().id.to_string()) {
                    error!("Error pushing txn to certified txns filter: {}", err);
                }
                (txn.txn().id(), txn.clone())
            })
            .collect();

        ProposalBlock::build(
            ref_hash,
            round,
            epoch,
            txns_list,
            claim_list,
            claim,
            self.keypair.get_miner_secret_key(),
        )
    }

    async fn broadcast_proposal_block(&self, proposal_block: ProposalBlock) {
        let event = Event::BlockCreated(Block::Proposal {
            block: proposal_block,
        });

        if let Err(err) = self.events_tx.send(event.into()).await {
            error!("{}", err);
        }
    }

    // The above code is handling an event of type `Vote` in a Rust
    // program. It checks the integrity of the vote by
    // verifying that it comes from the actual voter and prevents
    // double voting. It then adds the vote to a pool of votes for the
    // corresponding transaction and farmer quorum key. If
    // the number of votes in the pool reaches the farmer
    // quorum threshold, it sends a job to certify the transaction
    // using the provided signature provider.
    pub fn validate_vote(&self, vote: Vote, farmer_quorum_threshold: FarmerQuorumThreshold) {
        // TODO: Harvester quorum nodes should check the integrity of the vote by verifying the vote does
        // come from the alleged voter Node.
        //
        //     if let Some(sig_provider) = self.sig_provider.clone() {
        //         let farmer_quorum_key =
        // hex::encode(vote.quorum_public_key.clone());         if let
        // Some(mut votes) = self             .votes_pool
        //             .get_mut(&(vote.txn.id(), farmer_quorum_key.clone()))
        //         {
        //             let txn_id = vote.txn.id();
        //             if !self
        //                 .certified_txns_filter
        //                 .contains(&(txn_id.clone(),
        // farmer_quorum_key.clone()))             {
        //                 votes.push(vote.clone());
        //                 if votes.len() >= farmer_quorum_threshold {
        //                     let _ =
        // self.sync_jobs_sender.send(Job::CertifyTxn((
        // sig_provider,                         votes.clone(),
        //                         txn_id,
        //                         farmer_quorum_key,
        //                         vote.farmer_id.clone(),
        //                         vote.txn,
        //                         farmer_quorum_threshold,
        //                     )));
        //                 }
        //             }
        //         } else {
        //             self.votes_pool
        //                 .insert((vote.txn.id(), farmer_quorum_key),
        // vec![vote]);         }
        //     }
    }

    async fn broadcast_block_certificate(&self) {
        //
    }

    fn generate_and_broadcast_certificate(
        &self,
        block_hash: BlockHash,
        certificates_share: &HashSet<(NodeIdx, ValidatorPublicKeyShare, RawSignature)>,
        sig_provider: &SignatureProvider,
    ) -> Result<()> {
        todo!()
        // if certificates_share.len() >= self.quorum_threshold {
        //     //Generate a new certificate for the block
        //     let mut sig_shares = BTreeMap::new();
        //     certificates_share
        //         .iter()
        //         .for_each(|(node_idx, _, signature)| {
        //             sig_shares.insert(*node_idx, signature.clone());
        //         });
        //     if let Ok(certificate) =
        //         sig_provider.generate_quorum_signature(self.quorum_threshold
        // as u16, sig_shares)     {
        //         let certificate = Certificate {
        //             signature: hex::encode(certificate),
        //             inauguration: None,
        //             root_hash: "".to_string(),
        //             next_root_hash: "".to_string(),
        //             block_hash,
        //         };
        //
        //         self.broadcast_events_tx
        //             .send(EventMessage::new(
        //                 None,
        //                 Event::SendBlockCertificate(certificate),
        //             ))
        //             .await
        //             .map_err(|err| {
        //                 theater::TheaterError::Other(format!(
        //                     "failed to send block certificate: {err}"
        //                 ))
        //             })?
        //     }
        // }
        // Ok(())
    }

    async fn sign_convergence_block(&self) {
        //     Event::SignConvergenceBlock(block) => {
        //         if let Some(sig_provider) = self.sig_provider.clone() {
        //             let _ = self
        //                 .sync_jobs_sender
        //                 .send(Job::SignConvergenceBlock(sig_provider,
        // block));         }
        //     },
    }

    async fn process_convergence_block_partial_signature(&self) {
        //     // Process the job result of signing convergence block and adds
        // the     // partial signature to the cache for certificate
        // generation     Event::ConvergenceBlockPartialSign(job_result)
        // => {         if let JobResult::ConvergenceBlockPartialSign(
        //             block_hash,
        //             public_key_share,
        //             partial_signature,
        //         ) = job_result
        //         {
        //             if let Some(certificates_share) =
        //                 self.convergence_block_certificates.get(&block_hash)
        //             {
        //                 let mut new_certificate_share =
        // certificates_share.clone();                 if let
        // Ok(block_hash_bytes) = hex::decode(block_hash.clone()) {
        //                     if let Ok(signature) =
        //                         TryInto::<[u8;
        // 96]>::try_into(partial_signature.clone())
        // {                         if let Ok(signature_share) =
        // SignatureShare::from_bytes(signature) {
        // if public_key_share.verify(&signature_share, block_hash_bytes) {
        //                                 new_certificate_share.insert((
        //                                     self.harvester_id,
        //                                     public_key_share,
        //                                     partial_signature.clone(),
        //                                 ));
        //
        // self.convergence_block_certificates.push(
        // block_hash.clone(),
        // new_certificate_share.clone(),
        // );                                 if let Some(sig_provider)
        // = self.sig_provider.as_ref() {
        // if new_certificate_share.len()
        // <= sig_provider.quorum_config.upper_bound as usize
        //                                     {
        //                                         self
        //                                             .broadcast_events_tx
        //                                             .send(EventMessage::new(
        //                                                 None,
        //
        // Event::SendPeerConvergenceBlockSign(
        // self.harvester_id,
        // block_hash.clone(),
        // public_key_share.to_bytes().to_vec(),
        // partial_signature,
        // ),                                             ))
        //                                             .await.map_err(|err|
        // theater::TheaterError::Other(
        // format!("failed to send peer convergence block sign: {err}")
        //                                             ))?;
        //
        //
        // self.generate_and_broadcast_certificate(
        // block_hash,
        // &new_certificate_share,
        // sig_provider,                                         )
        //                                         .await?;
        //                                     }
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     },
    }

    //
    //     Event::PeerConvergenceBlockSign(
    //         node_idx,
    //         block_hash,
    //         public_key_share_bytes,
    //         partial_signature,
    //     ) => {
    //         let mut pb_key_share = None;
    //         let preliminary_check = TryInto::<[u8;
    // 48]>::try_into(public_key_share_bytes)
    // .and_then(|public_key_share_bytes| {
    // PublicKeyShare::from_bytes(public_key_share_bytes).map_err(|e| {
    //                     format!("Invalid Public Key, Expected 48byte array:
    // {e}").into_bytes()                 })
    //             })
    //             .and_then(|public_key_share| {
    //                 pb_key_share = Some(public_key_share);
    //                 TryInto::<[u8; 96]>::try_into(partial_signature.clone())
    //                     .and_then(|signature_share_bytes| {
    //
    // SignatureShare::from_bytes(signature_share_bytes).map_err(|e| {
    //                             format!("Invalid Signature, Expected 96byte
    // array: {e}")                                 .into_bytes()
    //                         })
    //                     })
    //                     .and_then(|signature_share| {
    //                         hex::decode(block_hash.clone())
    //                             .map_err(|e| {
    //                                 format!(
    //                                     "Invalid Hex Representation of Signature
    // Share: {e}",                                 )
    //                                 .into_bytes()
    //                             })
    //                             .and_then(|block_hash_bytes| {
    //                                 if public_key_share
    //                                     .verify(&signature_share,
    // block_hash_bytes)                                 {
    //                                     Ok(())
    //                                 } else {
    //                                     Err("signature verification failed"
    //                                         .to_string()
    //                                         .into_bytes())
    //                                 }
    //                             })
    //                     })
    //             });
    //
    //         if preliminary_check.is_ok() {
    //             if let Some(certificates_share) =
    //                 self.convergence_block_certificates.get(&block_hash)
    //             {
    //                 let mut new_certificate_share = certificates_share.clone();
    //                 if let Some(pb_key_share) = pb_key_share {
    //                     new_certificate_share.insert((
    //                         node_idx,
    //                         pb_key_share,
    //                         partial_signature,
    //                     ));
    //                     self.convergence_block_certificates
    //                         .push(block_hash.clone(),
    // new_certificate_share.clone());                     if let
    // Some(sig_provider) = self.sig_provider.as_ref() {
    // self.generate_and_broadcast_certificate(
    // block_hash,                             &new_certificate_share,
    //                             sig_provider,
    //                         )
    //                         .await?;
    //                     }
    //                 }
    //             }
    //         }
    //     },
    //     Event::PrecheckConvergenceBlock(block, last_confirmed_block_header) => {
    //         let claims = block.claims.clone();
    //         let txns = block.txns.clone();
    //         let proposal_block_hashes = block.header.ref_hashes.clone();
    //         let mut pre_check = true;
    //         let mut tmp_proposal_blocks = Vec::new();
    //         if let Ok(dag) = self.dag.read() {
    //             for proposal_block_hash in proposal_block_hashes.iter() {
    //                 if let Some(block) =
    // dag.get_vertex(proposal_block_hash.clone()) {                     if let
    // Block::Proposal { block } = block.get_data() {
    // tmp_proposal_blocks.push(block.clone());                     }
    //                 }
    //             }
    //             for (ref_hash, claim_hashset) in claims.iter() {
    //                 match dag.get_vertex(ref_hash.clone()) {
    //                     Some(block) => {
    //                         if let Block::Proposal { block } = block.get_data() {
    //                             for claim_hash in claim_hashset.iter() {
    //                                 if !block.claims.contains_key(claim_hash) {
    //                                     pre_check = false;
    //                                     break;
    //                                 }
    //                             }
    //                         }
    //                     },
    //                     None => {
    //                         pre_check = false;
    //                         break;
    //                     },
    //                 }
    //             }
    //             if pre_check {
    //                 for (ref_hash, txn_digest_set) in txns.iter() {
    //                     match dag.get_vertex(ref_hash.clone()) {
    //                         Some(block) => {
    //                             if let Block::Proposal { block } =
    // block.get_data() {                                 for txn_digest in
    // txn_digest_set.iter() {                                     if
    // !block.txns.contains_key(txn_digest) {
    // pre_check = false;                                         break;
    //                                     }
    //                                 }
    //                             }
    //                         },
    //                         None => {
    //                             pre_check = false;
    //                             break;
    //                         },
    //                     }
    //                 }
    //             }
    //         }
    //         if pre_check {
    //             self.broadcast_events_tx
    //                 .send(EventMessage::new(
    //                     None,
    //                     Event::CheckConflictResolution((
    //                         tmp_proposal_blocks,
    //                         last_confirmed_block_header.round,
    //                         last_confirmed_block_header.next_block_seed,
    //                         block,
    //                     )),
    //                 ))
    //                 .await
    //                 .map_err(|err| {
    //                     theater::TheaterError::Other(format!(
    //                         "failed to send conflict resolution check: {err}"
    //                     ))
    //                 })?
    //         }
    //     },
    //     Event::NoOp => {},
    //     _ => {},
    // }

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

        // TODO: publish a status update to mempool

        // let txn_record_opt = self.tx_mempool.get(&txn_id);
        // if let Some(mut txn_record) = txn_record_opt {
        //     txn_record.status = status;
        //     self.remove_txn(txn_id);
        //     self.insert_txn(txn_record.txn);
        // }
    }

    // Event  "Farm" fetches a batch of transactions from a transaction mempool and
    // sends them to scheduler to get it validated and voted
    pub fn farm_transactions(&mut self, transactions: Vec<(TransactionDigest, TxnRecord)>) {
        //
        // let keys: Vec<GroupPublicKey> = self
        //     .neighbouring_farmer_quorum_peers
        //     .keys()
        //     .cloned()
        //     .collect();

        // let maglev_hash_ring = Maglev::new(keys);
        //
        //     let mut new_txns = vec![];
        //
        //     for txn in txns.into_iter() {
        //         if let Some(group_public_key) =
        // maglev_hash_ring.get(&txn.0.clone()).cloned() {
        // if group_public_key == self.group_public_key {
        // new_txns.push(txn);             } else if let
        // Some(broadcast_addresses) =
        // self.neighbouring_farmer_quorum_peers.get(&group_public_key)
        //             {
        //                 let addresses: Vec<SocketAddr> =
        // broadcast_addresses.iter().cloned().collect();
        //
        //                 self.broadcast_events_tx
        //                     .send(EventMessage::new(
        //                         None,
        //                         Event::ForwardTxn((txn.1.clone(),
        // addresses.clone())),                     ))
        //                     .await
        //                     .map_err(|err| {
        //                         theater::TheaterError::Other(format!(
        //                             "failed to forward txn {:?} to peers
        // {addresses:?}:     {err}",
        //                             txn.1
        //                         ))
        //                     })?
        //             }
        //         } else {
        //             new_txns.push(txn);
        //         }
        //     }
        //
        //     if let Some(sig_provider) = self.sig_provider.clone() {
        //         if let Err(err) = self.sync_jobs_sender.send(Job::Farm((
        //             new_txns,
        //             self.farmer_id.clone(),
        //             self.farmer_node_idx,
        //             self.group_public_key.clone(),
        //             sig_provider,
        //             self.quorum_threshold,
        //         ))) {
        //             telemetry::error!("error sending job to scheduler: {}",
        // err);         }
        //     }
    }

    pub fn generate_partial_commitment_message(&mut self) -> Result<(Part, NodeId)> {
        let threshold_config = self.dkg_engine.threshold_config();

        let quorum_membership_config = self.quorum_driver.membership_config.clone().ok_or({
            error!("Cannot participate in DKG");
            NodeError::Other("Cannot participate in DKG".to_string())
        })?;

        let quorum_kind = quorum_membership_config.quorum_kind();

        let threshold = threshold_config.threshold as usize;

        // NOTE: add this node's own validator key to participate in DKG, otherwise they're considered
        // an observer and no part message is generated
        self.dkg_engine.add_peer_public_key(
            self.node_config.id.clone(),
            self.validator_public_key_owned(),
        );

        self.dkg_engine
            .generate_partial_commitment(threshold)
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    pub fn add_peer_public_key_to_dkg_state(
        &mut self,
        node_id: NodeId,
        public_key: ValidatorPublicKey,
    ) {
        self.dkg_engine.add_peer_public_key(node_id, public_key);
    }
}

impl<S: StateReader + Send + Sync + Clone> ConsensusModule<S> {
    pub async fn handle_node_added_to_peer_list(&mut self, peer_data: PeerData) -> Result<()> {
        if let Some(quorum_config) = self.quorum_driver.bootstrap_quorum_config.clone() {
            let node_id = peer_data.node_id.clone();

            let quorum_member_ids = quorum_config
                .membership_config
                .quorum_members
                .iter()
                .cloned()
                .map(|member| member.node_id)
                .collect::<Vec<NodeId>>();

            if quorum_member_ids.contains(&node_id) {
                self.quorum_driver
                    .bootstrap_quorum_available_nodes
                    .insert(node_id, (peer_data, true));
            }

            let available_nodes = self.quorum_driver.bootstrap_quorum_available_nodes.clone();

            let all_nodes_available = available_nodes.iter().all(|(_, (_, is_online))| *is_online);

            if all_nodes_available {
                telemetry::info!(
                    "All quorum members are online. Triggering genesis quorum elections"
                );

                if matches!(
                    self.quorum_driver.node_config.node_type,
                    primitives::NodeType::Bootstrap
                ) {
                    self.quorum_driver
                        .assign_peer_list_to_quorums(available_nodes)
                        .await?;
                }
            }
        }

        Ok(())
    }

    pub fn handle_quorum_membership_assigment_created(
        &mut self,
        assigned_membership: AssignedQuorumMembership,
    ) {
        let quorum_kind = assigned_membership.quorum_kind.clone();
        let quorum_membership_config = QuorumMembershipConfig {
            quorum_members: assigned_membership
                .peers
                .into_iter()
                .map(|peer| {
                    QuorumMember {
                        node_id: peer.node_id,
                        kademlia_peer_id: peer.kademlia_peer_id,
                        // TODO: get from kademlia metadata
                        node_type: NodeType::Validator,
                        udp_gossip_address: peer.udp_gossip_addr,
                        raptorq_gossip_address: peer.raptorq_gossip_addr,
                        kademlia_liveness_address: peer.kademlia_liveness_addr,

                        // TODO: create threshold signature keys for all nodes aand then share as
                        // part of membership config
                        // Then create a threshold signature from a harvester module masternode and
                        // use that to certify blocks
                        //
                        validator_public_key: self.node_config.keypair.validator_public_key_owned(),
                    }
                })
                .collect(),
            quorum_kind,
        };

        self.quorum_driver.membership_config = Some(quorum_membership_config);
    }

    pub fn handle_transaction_certificate_requested(
        &mut self,
        votes: Vec<Vote>,
        txn_id: TransactionDigest,
        quorum_key: PublicKeyShareVec,
        farmer_id: NodeId,
        txn: Txn,
        quorum_threshold: FarmerQuorumThreshold,
    ) {
        todo!()
        // let mut vote_shares: HashMap<bool, BTreeMap<NodeIdx, Vec<u8>>> =
        //     HashMap::new();
        // for v in votes.iter() {
        //     if let Some(votes) = vote_shares.get_mut(&v.is_txn_valid) {
        //         votes.insert(v.farmer_node_id, v.signature.clone());
        //     } else {
        //         let sig_shares_map: BTreeMap<NodeIdx, Vec<u8>> =
        //             vec![(v.farmer_node_id, v.signature.clone())]
        //                 .into_iter()
        //                 .collect();
        //         vote_shares.insert(v.is_txn_valid, sig_shares_map);
        //     }
        // }
        //
        // let validated_txns: Vec<_> = self
        //     .validator_core_manager
        //     .validate(
        //         &self.vrrbdb_read_handle.state_store_values(),
        //         vec![txn.clone()],
        //     )
        //     .into_iter()
        //     .collect();
        // let validated = validated_txns.par_iter().any(|x| x.0.id() == txn.id());
        // let most_votes_share = vote_shares
        //     .iter()
        //     .max_by_key(|(_, votes_map)| votes_map.len())
        //     .map(|(key, votes_map)| (*key, votes_map.clone()));
        // if validated {
        //     if let Some((is_txn_valid, votes_map)) = most_votes_share {
        //         let result = sig_provider.generate_quorum_signature(
        //             farmer_quorum_threshold as u16,
        //             votes_map.clone(),
        //         );
        //         if let Ok(threshold_signature) = result {
        //             self.events_tx
        //                 .send(
        //                     Event::CertifiedTxn(JobResult::CertifiedTxn(
        //                         votes.clone(),
        //                         threshold_signature,
        //                         txn_id.clone(),
        //                         farmer_quorum_key.clone(),
        //                         farmer_id.clone(),
        //                         Box::new(txn.clone()),
        //                         is_txn_valid,
        //                     ))
        //                     .into(),
        //                 )
        //                 .await
        //                 .map_err(|err| {
        //                     NodeError::Other(format!(
        //                         "failed to send certified txn: {err}"
        //                     ))
        //                 })?
        //         } else {
        //             error!("Quorum signature generation failed");
        //         }
        //     }
        // } else {
        //     error!("Penalize Farmer for wrong votes by sending Wrong Vote event to CR Quorum");
        // }
    }

    pub fn handle_transaction_certificate_created(
        &mut self,
        votes: Vec<Vote>,
        signature: RawSignature,
        digest: TransactionDigest,
        execution_result: ProgramExecutionOutput,
        farmer_id: NodeId,
        txn: Box<Txn>,
        is_valid: TxnValidationStatus,
    ) {
        // if let JobResult::CertifiedTxn(
        //     votes,
        //     certificate,
        //     txn_id,
        //     farmer_quorum_key,
        //     farmer_id,
        //     txn,
        //     is_txn_valid,
        // ) = job_result
        // {
        //     let vote_receipts = votes
        //         .iter()
        //         .map(|v| VoteReceipt {
        //             farmer_id: v.farmer_id.clone(),
        //             farmer_node_id: v.farmer_node_id,
        //             signature: v.signature.clone(),
        //         })
        //         .collect::<Vec<VoteReceipt>>();
        //
        //     self.quorum_certified_txns.push(QuorumCertifiedTxn::new(
        //         farmer_id,
        //         vote_receipts,
        //         *txn,
        //         certificate,
        //         is_txn_valid,
        //     ));
        //
        //     let _ = self
        //         .certified_txns_filter
        //         .push(&(txn_id, farmer_quorum_key));
        // }
    }

    pub fn handle_part_commitment_created(&mut self, node_id: NodeId, part: Part) {
        dbg!("handle_part_commitment_created");
        self.dkg_engine
            .dkg_state
            .part_message_store_mut()
            .entry(node_id)
            .or_insert_with(|| part);
    }

    pub fn handle_part_commitment_acknowledged(&mut self, node_id: NodeId) -> Result<()> {
        if self
            .dkg_engine
            .dkg_state
            .part_message_store_mut()
            .contains_key(&node_id)
        {
            self.dkg_engine.ack_partial_commitment(node_id)?;

            // PartMessageAcknowledged => {
            //                     if let Some(ack) = self
            //                         .dkg_engine
            //                         .dkg_state
            //                         .ack_message_store
            //                         .get(&(sender_id, self.dkg_engine.node_idx))
            //                     {
            //                         if let Ok(ack_bytes) = bincode::serialize(&ack) {
            //                             let event = Event::SendAck(
            //                                 self.dkg_engine.node_idx,
            //                                 sender_id,
            //                                 ack_bytes,
            //                             );
            //
            //                             let _ =
            // self.broadcast_events_tx.send(event.into()).await.map_err(|e| {
            //                                 error!("Error occured while sending ack message to
            // broadcast event channel {:?}", e);
            // TheaterError::Other(format!("{e:?}"))                             });
            //                         };
            //                     }
            // },
        }
        Ok(())
    }

    pub fn handle_quorum_election_started(&mut self, header: BlockHeader) {

        //     let claims = self.vrrbdb_read_handle.claim_store_values();
        //
        //     if let Ok(quorum) = self.elect_quorum(claims, header) {
        //         if let Err(err) = self
        //             .events_tx
        //             .send(Event::ElectedQuorum(quorum).into())
        //             .await
        //         {
        //             telemetry::error!("{}", err);
        //         }
        //     }
    }

    pub fn handle_miner_election_started(&mut self, header: BlockHeader) {
        // let claims = self.vrrbdb_read_handle.claim_store_values();
        // let mut election_results: BTreeMap<U256, Claim> =
        //     self.elect_miner(claims, header.block_seed);
        //
        //     let winner = Self::get_winner(&mut election_results);
        //
        //     if let Err(err) = self
        //         .events_tx
        //         .send(Event::ElectedMiner(winner).into())
        //         .await
        //     {
        //         telemetry::error!("{}", err);
        //     }
    }

    pub fn handle_txns_ready_for_processing(&mut self, txns: Vec<Txn>) {
        //     let txns = self.tx_mempool.fetch_txns(PULL_TXN_BATCH_SIZE);
        //     let keys: Vec<GroupPublicKey> = self
        //         .neighbouring_farmer_quorum_peers
        //         .keys()
        //         .cloned()
        //         .collect();
        //
        //     let maglev_hash_ring = Maglev::new(keys);
        //
        //     let mut new_txns = vec![];
        //
        //     for txn in txns.into_iter() {
        //         if let Some(group_public_key) = maglev_hash_ring.get(&txn.0.clone()).cloned()
        // {             if group_public_key == self.group_public_key {
        //                 new_txns.push(txn);
        //             } else if let Some(broadcast_addresses) =
        //                 self.neighbouring_farmer_quorum_peers.get(&group_public_key)
        //             {
        //                 let addresses: Vec<SocketAddr> =
        //                     broadcast_addresses.iter().cloned().collect();
        //
        //                 self.broadcast_events_tx
        //                     .send(EventMessage::new(
        //                         None,
        //                         Event::ForwardTxn((txn.1.clone(), addresses.clone())),
        //                     ))
        //                     .await
        //                     .map_err(|err| {
        //                         theater::TheaterError::Other(format!(
        //                             "failed to forward txn {:?} to peers {addresses:?}:
        // {err}",                             txn.1
        //                         ))
        //                     })?
        //             }
        //         } else {
        //             new_txns.push(txn);
        //         }
        //     }
        //
        //     if let Some(sig_provider) = self.sig_provider.clone() {
        //         if let Err(err) = self.sync_jobs_sender.send(Job::Farm((
        //             new_txns,
        //             self.farmer_id.clone(),
        //             self.farmer_node_idx,
        //             self.group_public_key.clone(),
        //             sig_provider,
        //             self.quorum_threshold,
        //         ))) {
        //             telemetry::error!("error sending job to scheduler: {}", err);
        //         }
        //     }
    }

    pub fn handle_proposal_block_mine_request_created(
        &mut self,
        ref_hash: RefHash,
        round: Round,
        epoch: Epoch,
        claim: Claim,
    ) {
        //     let txns = self.quorum_certified_txns.iter().take(PULL_TXN_BATCH_SIZE);
        //
        //     //Read updated claims
        //     let claim_map = self.vrrbdb_read_handle.claim_store_values();
        //     let claim_list = claim_map
        //         .values()
        //         .map(|claim| (claim.hash, claim.clone()))
        //         .collect();
        //
        //     let txns_list: LinkedHashMap<TransactionDigest, QuorumCertifiedTxn> = txns
        //         .into_iter()
        //         .map(|txn| {
        //             if let Err(err) =
        // self.certified_txns_filter.push(&txn.txn().id.to_string())             {
        //                 telemetry::error!(
        //                     "Error pushing txn to certified txns filter: {}",
        //                     err
        //                 );
        //             }
        //             (txn.txn().id(), txn.clone())
        //         })
        //         .collect();
        //
        //     let proposal_block = ProposalBlock::build(
        //         ref_hash,
        //         round,
        //         epoch,
        //         txns_list,
        //         claim_list,
        //         claim,
        //         self.keypair.get_miner_secret_key(),
        //     );
        //     let _ = self
        //         .broadcast_events_tx
        //         .send(EventMessage::new(
        //             None,
        //             Event::MinedBlock(Block::Proposal {
        //                 block: proposal_block,
        //             }),
        //         ))
        //         .await;
    }
    pub fn handle_convergence_block_partial_signature_created(
        &mut self,
        block_hash: BlockHash,
        public_key_share: ValidatorPublicKeyShare,
        partial_signature: RawSignature,
    ) {
        //         if let Some(certificates_share) =
        //             self.convergence_block_certificates.get(&block_hash)
        //         {
        //             let mut new_certificate_share = certificates_share.clone();
        //             if let Ok(block_hash_bytes) = hex::decode(block_hash.clone()) {
        //                 if let Ok(signature) =
        //                     TryInto::<[u8; 96]>::try_into(partial_signature.clone())
        //                 {
        //                     if let Ok(signature_share) =
        // SignatureShare::from_bytes(signature) {                         if
        // public_key_share.verify(&signature_share, block_hash_bytes) {
        // new_certificate_share.insert((
        // self.harvester_id,                                 public_key_share,
        //                                 partial_signature.clone(),
        //                             ));
        //                             self.convergence_block_certificates.push(
        //                                 block_hash.clone(),
        //                                 new_certificate_share.clone(),
        //                             );
        //                             if let Some(sig_provider) = self.sig_provider.as_ref() {
        //                                 if new_certificate_share.len()
        //                                     <= sig_provider.quorum_config.upper_bound as
        // usize                                 {
        //                                     self
        //                                         .broadcast_events_tx
        //                                         .send(EventMessage::new(
        //                                             None,
        //                                             Event::SendPeerConvergenceBlockSign(
        //                                                 self.harvester_id,
        //                                                 block_hash.clone(),
        //                                                 public_key_share.to_bytes().to_vec(),
        //                                                 partial_signature,
        //                                             ),
        //                                         ))
        //                                         .await.map_err(|err|
        // theater::TheaterError::Other(
        // format!("failed to send peer convergence block sign: {err}")
        // ))?;
        //
        //                                     self.generate_and_broadcast_certificate(
        //                                         block_hash,
        //                                         &new_certificate_share,
        //                                         sig_provider,
        //                                     )
        //                                     .await?;
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
    }
    pub fn handle_convergence_block_precheck_requested(
        &mut self,
        block: ConvergenceBlock,
        last_confirmed_block_header: BlockHeader,
    ) {
        //     let claims = block.claims.clone();
        //     let txns = block.txns.clone();
        //     let proposal_block_hashes = block.header.ref_hashes.clone();
        //     let mut pre_check = true;
        //     let mut tmp_proposal_blocks = Vec::new();
        //     if let Ok(dag) = self.dag.read() {
        //         for proposal_block_hash in proposal_block_hashes.iter() {
        //             if let Some(block) = dag.get_vertex(proposal_block_hash.clone()) {
        //                 if let Block::Proposal { block } = block.get_data() {
        //                     tmp_proposal_blocks.push(block.clone());
        //                 }
        //             }
        //         }
        //         for (ref_hash, claim_hashset) in claims.iter() {
        //             match dag.get_vertex(ref_hash.clone()) {
        //                 Some(block) => {
        //                     if let Block::Proposal { block } = block.get_data() {
        //                         for claim_hash in claim_hashset.iter() {
        //                             if !block.claims.contains_key(claim_hash) {
        //                                 pre_check = false;
        //                                 break;
        //                             }
        //                         }
        //                     }
        //                 },
        //                 None => {
        //                     pre_check = false;
        //                     break;
        //                 },
        //             }
        //         }
        //         if pre_check {
        //             for (ref_hash, txn_digest_set) in txns.iter() {
        //                 match dag.get_vertex(ref_hash.clone()) {
        //                     Some(block) => {
        //                         if let Block::Proposal { block } = block.get_data() {
        //                             for txn_digest in txn_digest_set.iter() {
        //                                 if !block.txns.contains_key(txn_digest) {
        //                                     pre_check = false;
        //                                     break;
        //                                 }
        //                             }
        //                         }
        //                     },
        //                     None => {
        //                         pre_check = false;
        //                         break;
        //                     },
        //                 }
        //             }
        //         }
        //     }
        //     if pre_check {
        //         self.broadcast_events_tx
        //             .send(EventMessage::new(
        //                 None,
        //                 Event::CheckConflictResolution((
        //                     tmp_proposal_blocks,
        //                     last_confirmed_block_header.round,
        //                     last_confirmed_block_header.next_block_seed,
        //                     block,
        //                 )),
        //             ))
        //             .await
        //             .map_err(|err| {
        //                 theater::TheaterError::Other(format!(
        //                     "failed to send conflict resolution check: {err}"
        //                 ))
        //             })?
        //     }
    }
    pub fn handle_convergence_block_peer_signature_request(
        &mut self,
        node_id: NodeId,
        block_hash: BlockHash,
        public_key_share: PublicKeyShareVec,
        partial_signature: RawSignature,
    ) {
        //     let mut pb_key_share = None;
        //     let preliminary_check = TryInto::<[u8; 48]>::try_into(public_key_share_bytes)
        //         .and_then(|public_key_share_bytes| {
        //             PublicKeyShare::from_bytes(public_key_share_bytes).map_err(|e| {
        //                 format!("Invalid Public Key, Expected 48byte array:
        // {e}").into_bytes()             })
        //         })
        //         .and_then(|public_key_share| {
        //             pb_key_share = Some(public_key_share);
        //             TryInto::<[u8; 96]>::try_into(partial_signature.clone())
        //                 .and_then(|signature_share_bytes| {
        //                     SignatureShare::from_bytes(signature_share_bytes).map_err(|e| {
        //                         format!("Invalid Signature, Expected 96byte array: {e}")
        //                             .into_bytes()
        //                     })
        //                 })
        //                 .and_then(|signature_share| {
        //                     hex::decode(block_hash.clone())
        //                         .map_err(|e| {
        //                             format!(
        //                                 "Invalid Hex Representation of Signature Share: {e}",
        //                             )
        //                             .into_bytes()
        //                         })
        //                         .and_then(|block_hash_bytes| {
        //                             if public_key_share
        //                                 .verify(&signature_share, block_hash_bytes)
        //                             {
        //                                 Ok(())
        //                             } else {
        //                                 Err("signature verification failed"
        //                                     .to_string()
        //                                     .into_bytes())
        //                             }
        //                         })
        //                 })
        //         });
        //
        //     if preliminary_check.is_ok() {
        //         if let Some(certificates_share) =
        //             self.convergence_block_certificates.get(&block_hash)
        //         {
        //             let mut new_certificate_share = certificates_share.clone();
        //             if let Some(pb_key_share) = pb_key_share {
        //                 new_certificate_share.insert((
        //                     node_idx,
        //                     pb_key_share,
        //                     partial_signature,
        //                 ));
        //                 self.convergence_block_certificates
        //                     .push(block_hash.clone(), new_certificate_share.clone());
        //                 if let Some(sig_provider) = self.sig_provider.as_ref() {
        //                     self.generate_and_broadcast_certificate(
        //                         block_hash,
        //                         &new_certificate_share,
        //                         sig_provider,
        //                     )
        //                     .await?;
        //                 }
        //             }
        //         }
        //     }
        //
    }
}
