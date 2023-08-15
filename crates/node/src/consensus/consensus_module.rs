use std::collections::HashSet;

use block::{Block, ProposalBlock, RefHash};
use chrono::Duration;
use events::{Event, EventMessage, EventPublisher, EventSubscriber, SyncPeerData, Vote};
use hbbft::crypto::{PublicKeyShare, SecretKeyShare};
use laminar::{Packet, SocketEvent};
use maglev::Maglev;
use mempool::{TxnRecord, TxnStatus};
use primitives::{
    BlockHash,
    Epoch,
    FarmerQuorumThreshold,
    GroupPublicKey,
    NodeIdx,
    NodeTypeBytes,
    PKShareBytes,
    PayloadBytes,
    QuorumPublicKey,
    RawSignature,
    Round,
};
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use signer::signer::SignatureProvider;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorState};
use vrrb_core::{
    bloom::Bloom,
    claim::Claim,
    keypair::Keypair,
    txn::{QuorumCertifiedTxn, TransactionDigest, Txn},
};

use crate::{NodeError, RuntimeComponent, RuntimeComponentHandle};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

pub trait QuorumMember {}
// TODO: Move this to primitives
pub type QuorumId = String;
pub type QuorumPubkey = String;

#[derive(Debug, Clone)]
pub struct ConsensusModuleConfig {
    pub events_tx: EventPublisher,
    pub keypair: Keypair,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
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
pub struct ConsensusModule {
    pub(crate) id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) vrrbdb_read_handle: VrrbDbReadHandle,
    pub(crate) quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    pub(crate) keypair: Keypair,
    pub(crate) certified_txns_filter: Bloom,
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

impl ConsensusModule {
    pub fn new(cfg: ConsensusModuleConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ActorState::Stopped,
            events_tx: cfg.events_tx,
            vrrbdb_read_handle: cfg.vrrbdb_read_handle,
            quorum_certified_txns: vec![],
            keypair: cfg.keypair,
            certified_txns_filter: Bloom::new(10),
        }
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
                    telemetry::error!("Error pushing txn to certified txns filter: {}", err);
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
            telemetry::error!("{}", err);
        }
    }

    async fn ceritfy_transaction(&self) {
        // // This certifies txns once vote threshold is reached.
        // // Event::CertifiedTxn(job_result) => {
        //     if let JobResult::CertifiedTxn(
        //         votes,
        //         certificate,
        //         txn_id,
        //         farmer_quorum_key,
        //         farmer_id,
        //         txn,
        //         is_txn_valid,
        //     ) = job_result
        //     {
        //         let vote_receipts = votes
        //             .iter()
        //             .map(|v| VoteReceipt {
        //                 farmer_id: v.farmer_id.clone(),
        //                 farmer_node_id: v.farmer_node_id,
        //                 signature: v.signature.clone(),
        //             })
        //             .collect::<Vec<VoteReceipt>>();
        //         self.quorum_certified_txns.push(QuorumCertifiedTxn::new(
        //             farmer_id,
        //             vote_receipts,
        //             *txn,
        //             certificate,
        //             is_txn_valid,
        //         ));
        //         let _ = self
        //             .certified_txns_filter
        //             .push(&(txn_id, farmer_quorum_key));
        //     }
        // // },
    }

    // The above code is handling an event of type `Vote` in a Rust
    // program. It checks the integrity of the vote by
    // verifying that it comes from the actual voter and prevents
    // double voting. It then adds the vote to a pool of votes for the
    // corresponding transaction and farmer quorum key. If
    // the number of votes in the pool reaches the farmer
    // quorum threshold, it sends a job to certify the transaction
    // using the provided signature provider.
    fn validate_vote(&self, vote: Vote, farmer_quorum_threshold: FarmerQuorumThreshold) {
        //     //TODO Harvest should check for integrity of the vote by Voter(
        // Does it vote     // truly comes from Voter Prevent Double
        // Voting
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
        certificates_share: &HashSet<(NodeIdx, PublicKeyShare, RawSignature)>,
        sig_provider: &SignatureProvider,
    ) -> Result<(), theater::TheaterError> {
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

    //
    pub async fn process_rendezvous_response(&self) {
        // let receiver = self.socket.get_event_receiver();
        // let sender = self.socket.get_packet_sender();
        // loop {
        //     if let Ok(event) = receiver.recv() {
        //         self.process_rendezvous_event(&event, &sender).await
        //     }
        // }
    }

    pub fn send_register_retrieve_peers_request(&self) {
        // let sender = self.socket.get_packet_sender();
        //
        // let (tx1, rx1) = unbounded();
        // let (tx2, rx2) = unbounded();
        //
        // // Spawning threads for retrieve peers request and register request
        // DkgModule::spawn_interval_thread(Duration::from_secs(RETRIEVE_PEERS_REQUEST), tx1);
        //
        // DkgModule::spawn_interval_thread(Duration::from_secs(REGISTER_REQUEST), tx2);
        //
        // loop {
        //     select! {
        //         recv(rx1) -> _ => {
        //             self.send_retrieve_peers_request(
        //                 &sender
        //             );
        //         },
        //         recv(rx2) -> _ => {
        //             self.send_register_request(
        //                 &sender,
        //             );
        //         },
        //     }
        // }
    }

    async fn process_rendezvous_event(
        &self,
        event: &SocketEvent,
        // sender: &Sender<Packet>
    ) {
        // if let SocketEvent::Packet(packet) = event {
        //     self.process_packet(packet, sender).await;
        // }
    }

    async fn process_packet(
        &self,
        packet: &Packet,
        // sender: &Sender<Packet>
    ) {
        // if packet.addr() == self.rendezvous_server_addr {
        //     if let Ok(payload_response) =
        // bincode::deserialize::<Data>(packet.payload()) {
        //         self.process_payload_response(&payload_response, sender,
        // packet)             .await;
        //     }
        // }
    }

    async fn process_payload_response(
        &self,
        payload_response: &Data,
        // sender: &Sender<Packet>,
        // packet: &Packet,
    ) {
        // match payload_response {
        //     Data::Request(req) => self.process_request(req, sender, packet),
        //     Data::Response(resp) => self.process_response(resp).await,
        // }
    }

    fn process_request(
        &self,
        request: &RendezvousRequest,
        // sender: &Sender<Packet>,
        packet: &Packet,
    ) {
        // if let RendezvousRequest::Ping = request {
        //     let response = &Data::Response(RendezvousResponse::Pong);
        //     if let Ok(data) = bincode::serialize(&response) {
        //         let _ = sender.send(Packet::reliable_unordered(packet.addr(),
        // data));     }
        // };
    }

    async fn process_response(&self, response: &RendezvousResponse) {
        //     match response {
        //         RendezvousResponse::Peers(peers) => {
        //             let _ = self
        //                 .broadcast_events_tx
        //                 .send(Event::SyncPeers(peers.clone()).into())
        //                 .await;
        //         },
        //         RendezvousResponse::NamespaceRegistered => {
        //             info!("Namespace Registered");
        //         },
        //         RendezvousResponse::PeerRegistered => {
        //             info!("Peer Registered");
        //         },
        //         _ => {},
        //     }
    }

    //
    // fn send_retrieve_peers_request(&self, sender: &Sender<Packet>) {
    //     let quorum_key = if self.dkg_engine.node_type == NodeType::Farmer {
    //         self.dkg_engine.harvester_public_key
    //     } else {
    //         self.dkg_engine
    //             .dkg_state
    //             .public_key_set
    //             .as_ref()
    //             .map(|key| key.public_key())
    //     };
    //
    //     if let Some(harvester_public_key) = quorum_key {
    //         if let Ok(data) =
    // bincode::serialize(&Data::Request(RendezvousRequest::Peers(
    // harvester_public_key.to_bytes().to_vec(),         ))) {
    //             let _ = sender.send(Packet::reliable_ordered(
    //                 self.rendezvous_server_addr,
    //                 data,
    //                 None,
    //             ));
    //         }
    //     }
    // }
    //
    // fn send_register_request(&self, sender: &Sender<Packet>) {
    //     match self.dkg_engine.dkg_state.public_key_set.clone() {
    //         Some(quorum_key) => {
    //             self.send_namespace_registration(sender,
    // &quorum_key.public_key());
    //
    //             if let Some(secret_key_share) =
    // self.dkg_engine.dkg_state.secret_key_share.clone() {                 let
    // (msg_bytes, signature) = self.generate_random_payload(&secret_key_share);
    //                 self.send_register_peer_payload(
    //                     sender,
    //                     &secret_key_share,
    //                     msg_bytes,
    //                     signature,
    //                     &quorum_key.public_key(),
    //                 );
    //             }
    //         },
    //         None => {
    //             error!(
    //                 "Cannot proceed with registration since current node is not
    // part of any quorum"             );
    //         },
    //     }
    // }
    //
    // fn send_namespace_registration(&self, sender: &Sender<Packet>, quorum_key:
    // &PublicKey) {     if let Ok(data) =
    // bincode::serialize(&Data::Request(RendezvousRequest::Namespace(
    //         self.dkg_engine.node_type.to_string().as_bytes().to_vec(),
    //         quorum_key.to_bytes().to_vec(),
    //     ))) {
    //         let _ = sender.send(Packet::reliable_ordered(
    //             self.rendezvous_server_addr,
    //             data,
    //             None,
    //         ));
    //         thread::sleep(Duration::from_secs(5));
    //     }
    // }
    //
    // fn send_register_peer_payload(
    //     &self,
    //     sender: &Sender<Packet>,
    //     secret_key_share: &SecretKeyShare,
    //     msg_bytes: Vec<u8>,
    //     signature: Vec<u8>,
    //     quorum_key: &PublicKey,
    // ) {
    //     let payload_result =
    // bincode::serialize(&Data::Request(RendezvousRequest::RegisterPeer(
    //         quorum_key.to_bytes().to_vec(),
    //         self.dkg_engine.node_type.to_string().as_bytes().to_vec(),
    //         secret_key_share.public_key_share().to_bytes().to_vec(),
    //         signature,
    //         msg_bytes,
    //         SyncPeerData {
    //             address: self.rendezvous_local_addr,
    //             raptor_udp_port: self.rendezvous_local_addr.port(),
    //             quic_port: self.quic_port,
    //             node_type: self.dkg_engine.node_type,
    //         },
    //     )));
    //     if let Ok(payload) = payload_result {
    //         let _ = sender.send(Packet::reliable_ordered(
    //             self.rendezvous_server_addr,
    //             payload,
    //             None,
    //         ));
    //     }
    // }

    fn generate_random_payload(&self, secret_key_share: &SecretKeyShare) -> (Vec<u8>, Vec<u8>) {
        // let message: String = rand::thread_rng()
        //         .sample_iter(&Alphanumeric)
        //         .take(15)
        //         .map(char::from)
        //         .collect();
        //
        //     let msg_bytes = if let Ok(m) = hex::decode(message.clone()) {
        //         m
        //     } else {
        //         vec![]
        //     };
        //
        //     let signature = secret_key_share.sign(message).to_bytes().to_vec();
        //
        //     (msg_bytes, signature)
        todo!()
    }

    fn spawn_interval_thread(interval: Duration /* tx: Sender<()> */) {
        todo!()
        //     thread::spawn(move || loop {
        //         sleep(interval);
        //         let _ = tx.send(());
        //     });
    }

    // Event  "Farm" fetches a batch of transactions from a transaction mempool and
    // sends them to scheduler to get it validated and voted
    pub fn farm_transactions(&mut self, transactions: Vec<(TransactionDigest, TxnRecord)>) {
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

    pub fn handle_dkg_protocol_initiated(&self) {
        //     let threshold_config = self.dkg_engine.threshold_config.clone();
        //     if self.quorum_type.clone().is_some() {
        //         match self
        //             .dkg_engine
        //             .generate_sync_keygen_instance(threshold_config.threshold
        // as usize)         {
        //             Ok(part_commitment) => {
        //                 if let DkgResult::PartMessageGenerated(node_idx,
        // part) = part_commitment                 {
        //                     if let Ok(part_committment_bytes) =
        // bincode::serialize(&part) {                         let _ =
        // self                             .broadcast_events_tx
        //                             .send(
        //                                 Event::PartMessage(node_idx,
        // part_committment_bytes)
        // .into(),                             )
        //                             .await.map_err(|e| {
        //                                 error!("Error occured while sending
        // part message to broadcast event channel {:?}", e);
        // TheaterError::Other(format!("{e:?}"))                             });
        //                     }
        //                 }
        //             },
        //             Err(_e) => {
        //                 error!("Error occured while generating synchronized
        // keygen instance for node {:?}", self.dkg_engine.node_idx);
        // },         }
        //     } else {
        //         error!(
        //             "Cannot participate into DKG ,since current node {:?}
        // dint win any Quorum Election",
        // self.dkg_engine.node_idx         );
        //     }
        //     return Ok(ActorState::Running);
    }
}
