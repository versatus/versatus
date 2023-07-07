use std::collections::HashSet;

use block::{ProposalBlock, RefHash};
use events::Vote;
use hbbft::crypto::PublicKeyShare;
use primitives::{BlockHash, Epoch, FarmerQuorumThreshold, NodeIdx, RawSignature, Round};
use ritelinked::LinkedHashMap;
use signer::signer::SignatureProvider;
use storage::vrrbdb::VrrbDbReadHandle;
use vrrb_core::{
    bloom::Bloom,
    claim::Claim,
    keypair::Keypair,
    txn::{QuorumCertifiedTxn, TransactionDigest},
};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

#[derive(Debug)]
pub struct ConsensusModule {
    vrrbdb_read_handle: VrrbDbReadHandle,
    quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    keypair: Keypair,
    certified_txns_filter: Bloom,
    // votes_pool: DashMap<(TransactionDigest, String), Vec<Vote>>,
    // group_public_key: GroupPublicKey,
    // sig_provider: Option<SignatureProvider>,
    // vrrbdb_read_handle: VrrbDbReadHandle,
    // convergence_block_certificates:
    //     Cache<BlockHash, HashSet<(NodeIdx, PublicKeyShare, RawSignature)>>,
    //
    //
    // harvester_id: NodeIdx,
    // dag: Arc<RwLock<BullDag<Block, String>>>,
    // quorum_threshold: QuorumThreshold,
    //
    //
    // sync_jobs_sender: Sender<Job>,
    // status: ActorState,
    // id: ActorId,
    // broadcast_events_tx: EventPublisher,
    // _label: ActorLabel,
    // _events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
    // _async_jobs_sender: Sender<Job>,
}

impl ConsensusModule {
    async fn certify_block(&self) {}

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

    async fn broadcast_proposal_block(&self) {
        // move broadcasting to another function
        // let _ = self
        //     .broadcast_events_tx
        //     .send(EventMessage::new(
        //         None,
        //         Event::MinedBlock(Block::Proposal {
        //             block: proposal_block,
        //         }),
        //     ))
        //     .await;
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
}
