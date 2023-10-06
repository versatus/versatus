use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};
use block::{
    header::BlockHeader, Block, BlockHash, Certificate, ConvergenceBlock, GenesisBlock, ProposalBlock,
};
// use dkg_engine::{dkg::DkgGenerator, prelude::DkgEngine};
use bulldag::graph::BullDag;
//use dkg_engine::{dkg::DkgGenerator, prelude::DkgEngine};
use events::{SyncPeerData, Vote};
use hbbft::{
    crypto::{PublicKeySet, PublicKeyShare, SecretKeyShare},
    sync_key_gen::Part,
};
use indexmap::IndexMap;
use mempool::{TxnStatus, MempoolReadHandleFactory};
use miner::{conflict_resolver::Resolver, block_builder::BlockBuilder};
use primitives::{
    ByteVec, FarmerQuorumThreshold, GroupPublicKey, NodeId, NodeType, NodeTypeBytes, PKShareBytes,
    PayloadBytes, QuorumId, QuorumPublicKey, QuorumType, RawSignature, ValidatorPublicKey, Signature, PublicKey,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use signer::{
    engine::{SignerEngine, QuorumData, QuorumMembers},
    signer::{SignatureProvider, Signer},
};
use storage::vrrbdb::{ClaimStoreReadHandleFactory, StateStoreReadHandleFactory};
use telemetry::error;
use validator::validator_core_manager::ValidatorCoreManager;
use vrrb_config::{NodeConfig, QuorumMembershipConfig};
use vrrb_core::{bloom::Bloom, keypair::Keypair};
use vrrb_core::{
    cache::Cache,
    transactions::{QuorumCertifiedTxn, Transaction, TransactionDigest, TransactionKind},
};

use super::{QuorumModule, QuorumModuleConfig};
use crate::{NodeError, Result};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

// TODO: Move this to primitives

#[derive(Debug)]
pub struct ConsensusModuleConfig {
    pub keypair: Keypair,
    pub node_config: NodeConfig,
    // pub dkg_generator: DkgEngine,
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

#[derive(Debug, Clone)]
pub struct ConsensusModule {
    pub(crate) quorum_certified_txns: Vec<QuorumCertifiedTxn>,
    pub(crate) keypair: Keypair,
    pub(crate) certified_txns_filter: Bloom,
    pub(crate) quorum_driver: QuorumModule,
    pub(crate) sig_engine: SignerEngine,
    pub(crate) node_config: NodeConfig,
    pub(crate) quorum_membership: Option<QuorumId>,
    pub(crate) quorum_type: Option<QuorumType>,
    pub votes_pool: HashMap<QuorumId, HashMap<TransactionDigest, HashSet<Vote>>>,
    pub(crate) validator_core_manager: ValidatorCoreManager,
}

impl ConsensusModule {
    pub fn new(
        cfg: ConsensusModuleConfig,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
        claim_reader: ClaimStoreReadHandleFactory,
        cores: usize
    ) -> Result<Self> {
        let quorum_module_config = QuorumModuleConfig {
            membership_config: None,
            node_config: cfg.node_config.clone(),
        };

        let validator_public_key = cfg.keypair.validator_public_key_owned();

        let validator_core_manager =
            ValidatorCoreManager::new(cores, mempool_reader, state_reader, claim_reader).map_err(
                |err| NodeError::Other(format!("failed to generate validator core manager: {err}")),
            )?;

        let sig_engine = SignerEngine::new(
            cfg.keypair.get_miner_public_key().clone(),
            cfg.keypair.get_miner_secret_key().clone(),
        );

        Ok(Self {
            quorum_certified_txns: vec![],
            keypair: cfg.keypair,
            certified_txns_filter: Bloom::new(10),
            quorum_driver: QuorumModule::new(quorum_module_config),
            sig_engine,
            node_config: cfg.node_config.clone(),
            quorum_membership: None,
            quorum_type: None,
            validator_core_manager,
            votes_pool: Default::default(),
        })
    }

    pub fn validator_public_key_owned(&self) -> ValidatorPublicKey {
        self.keypair.validator_public_key_owned()
    }

    pub fn certify_block(
        &mut self,
        block: Block,
        last_block_header: BlockHeader,
        prev_txn_root_hash: String,
        next_txn_root_hash: String,
        certs: Vec<(NodeId, Signature)>,
    ) -> Result<Certificate> {
        let block = block.clone();
        let block_hash = block.hash();
        let quorum_threshold = self.node_config.threshold_config.threshold;

        if certs.len() as u16 <= quorum_threshold {
            return Err(NodeError::Other(
                "Not enough partial signatures to create a certificate".to_string(),
            ));
        }
       
        self.sig_engine.verify_batch(&certs, &block.hash())
            .map_err(|err| NodeError::Other(err.to_string()))?;

        //TODO: If Quorums are pending inauguration include inauguration info
        let certificate = Certificate {
            signatures: certs.clone(),
            inauguration: None,
            root_hash: prev_txn_root_hash,
            next_root_hash: next_txn_root_hash,
            block_hash,
        };

        Ok(certificate)
    }

    pub fn certify_genesis_block(&mut self, block: GenesisBlock, certs: Vec<(NodeId, Signature)>) -> Result<Certificate> {
        let txn_trie_hash = block.header.txn_hash.clone();
        let last_block_header = block.header.clone();

        self.certify_block(
            block.into(),
            last_block_header,
            txn_trie_hash.clone(),
            txn_trie_hash,
            certs
        )
    }

    pub fn certify_convergence_block<R: Resolver<Proposal = ProposalBlock>>(
        &mut self,
        block: ConvergenceBlock,
        last_block_header: BlockHeader,
        next_txn_root_hash: String,
        resolver: R,
        dag: Arc<RwLock<BullDag<Block, String>>>,
        certs: Vec<(NodeId, Signature)>
        // certificates_share: &HashSet<(NodeIdx, ValidatorPublicKeyShare, RawSignature)>,
    ) -> Result<Certificate> {
        let prev_txn_root_hash = last_block_header.txn_hash.clone();

        self.precheck_convergence_block(
            block.clone(), 
            last_block_header.clone(),
            resolver,
            dag.clone()
        );
        self.certify_block(
            block.into(),
            last_block_header,
            prev_txn_root_hash,
            next_txn_root_hash,
            certs
        )
    }

    async fn sign_convergence_block(
        &mut self,
        block: ConvergenceBlock,
    ) -> Result<(String, PublicKey, Signature)> {
        let block_hash_bytes = hex::decode(block.hash.clone())
            .map_err(|err| NodeError::Other(format!("unable to decode block hash: {err}")))?;

        let signature = self.sig_engine.sign(block_hash_bytes)
            .map_err(|err| {
                NodeError::Other(format!("failed to generate partial signature: {err}"))
            })?;

        Ok((
            block.hash.clone(),
            self.sig_engine.public_key(),
            signature.clone(),
        ))
    }
    
    #[deprecated]
    pub fn generate_partial_commitment_message(&mut self) -> Result<(Part, NodeId)> {
        if self.node_config.node_type == NodeType::Bootstrap {
            return Err(NodeError::Other(
                "Bootstrap nodes cannot participate in DKG".to_string(),
            ));
        }

        if self.node_config.node_type == NodeType::Miner {
            return Err(NodeError::Other(
                "Miner nodes cannot participate in Validator DKG".to_string(),
            ));
        }

        let quorum_membership_config = self.quorum_driver.membership_config.clone().ok_or({
            let err_msg = format!("Node {} cannot participate in DKG", self.node_config.id);
            error!(err_msg);
            NodeError::Other(err_msg)
        })?;

        let threshold = quorum_membership_config.quorum_members().len() / 2;

        // NOTE: add this node's own validator key to participate in DKG, otherwise they're considered
        // an observer and no part message is generated
        // self.dkg_engine.add_peer_public_key(
        //     self.node_config.id.clone(),
        //     self.validator_public_key_owned(),
        // );

        // self.dkg_engine
        //     .generate_partial_commitment(threshold)
        //     .map_err(|err| NodeError::Other(err.to_string()))
        todo!()
    }

    // pub fn add_peer_public_key_to_dkg_state(
    //     &mut self,
    //     node_id: NodeId,
    //     public_key: ValidatorPublicKey,
    // ) {
    //     self.dkg_engine.add_peer_public_key(node_id, public_key);
    // }

    pub fn membership_config(&self) -> &Option<QuorumMembershipConfig> {
        &self.quorum_driver.membership_config
    }

    pub fn membership_config_mut(&mut self) -> &mut Option<QuorumMembershipConfig> {
        &mut self.quorum_driver.membership_config
    }

    pub fn membership_config_owned(&self) -> Result<QuorumMembershipConfig> {
        let quorum_membership_config =
            self.quorum_driver
                .membership_config
                .clone()
                .ok_or(NodeError::Other(
                    "failed to read quorum configuration".into(),
                ))?;

        Ok(quorum_membership_config)
    }

    // pub fn quorum_public_keyset(&self) -> Result<PublicKeySet> {
    //     let dkg_state = &self.dkg_engine.dkg_state;
    //     let public_keyset = self
    //         .dkg_engine
    //         .dkg_state
    //         .public_key_set_owned()
    //         .ok_or(NodeError::Other("failed to read public key set".into()))?;

    //     Ok(public_keyset)
    // }

    // pub fn quorum_secret_key_share(&self) -> Result<SecretKeyShare> {
    //     let secret_key_share = self
    //         .dkg_engine
    //         .dkg_state
    //         .secret_key_share_owned()
    //         .ok_or(NodeError::Other("failed to read secret key share".into()))?;

    //     Ok(secret_key_share)
    // }

    pub fn validate_transaction_kind(
        &mut self,
        digest: &TransactionDigest,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
    ) -> validator::txn_validator::Result<TransactionKind> {
        self.validator_core_manager
            .validate_transaction_kind(digest, mempool_reader, state_reader)
    }

    #[deprecated]
    pub fn validate_transactions(
        &mut self,
        txns: Vec<TransactionKind>,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
    ) -> HashSet<(TransactionKind, validator::txn_validator::Result<()>)> {
        self.validator_core_manager
            .validate(txns, mempool_reader, state_reader)
            .into_iter()
            .collect()
    }

    pub fn cast_vote_on_transaction_kind(
        &mut self,
        transaction: TransactionKind,
        valid: bool,
    ) -> Result<Vote> {
        // NOTE: comments originally by vsawant, check with them to figure out what they meant
        //
        // TODO  Add Delegation logic + Handling Double Spend by checking whether
        // MagLev Hashing over( Quorum Keys) to identify whether current farmer
        // quorum is supposed to vote on txn Txn is intended
        // to be validated by current validator
        //
        // let _backpressure = self.job_scheduler.calculate_back_pressure();
        // Delegation Principle need to be done

        // let farmer_quorum_threshold = self.quorum_public_keyset()?.threshold();
        // let quorum_public_key = self
        //     .quorum_public_keyset()?
        //     .public_key()
        //     .to_bytes()
        //     .to_vec();

        if let Some(vote) = self.form_vote(transaction, valid) {
            return Ok(vote);
        }

        // TODO: Return the transaction id in the error for better
        // error handling
        Err(NodeError::Other(format!(
            "could not produce vote on transaction"
        )))
    }

    fn form_vote(&mut self, transaction: TransactionKind, valid: bool) -> Option<Vote> {
        let receiver_farmer_id = self.node_config.id.clone();
        let farmer_node_id = self.node_config.id.clone();

        let txn_bytes = bincode::serialize(&transaction.clone()).ok()?;
        let signature = self.sig_engine.sign(txn_bytes).ok()?;

        Some(Vote {
            farmer_id: receiver_farmer_id.clone().into(),
            farmer_node_id: farmer_node_id.clone(),
            signature,
            txn: transaction.clone(),
            execution_result: None,
            is_txn_valid: valid,
        })
    }

    fn validate_single_transaction(
        &mut self,
        txn: &TransactionKind,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
    ) -> bool {
        let validated_txns =
            self.validator_core_manager
                .validate(vec![txn.clone()], mempool_reader, state_reader);

        validated_txns.iter().any(|x| x.0.id() == txn.id())
    }

    pub fn insert_vote_into_vote_pool(&mut self, vote: Vote) -> Result<()> {
        self.is_quorum_member()?;
        self.is_harvester()?;

        if let Some((quorum_id, QuorumType::Farmer)) = self.get_node_quorum_id(&vote.farmer_id) {
            let nested_map = self
                .votes_pool
                .entry(quorum_id)
                .or_insert_with(HashMap::new);
            let digest = vote.txn.id();
            let vote_set = nested_map.entry(digest).or_insert_with(HashSet::new);
            vote_set.insert(vote);
            return Ok(());
        }

        return Err(NodeError::Other(format!(
            "node is not a member of currently active quorum"
        )));
    }

    fn get_node_quorum_id(&self, node_id: &NodeId) -> Option<(QuorumId, QuorumType)> {
        let quorum_members = self.sig_engine.quorum_members();
        for (quorum_id, quorum_data) in quorum_members.0.iter() {
            if quorum_data.members.contains_key(node_id) {
                return Some((quorum_id.clone(), quorum_data.quorum_type.clone()));
            }
        }
        None
    }

    fn is_quorum_member(&self) -> Result<()> {
        if self.quorum_membership.is_none() {
            return Err(NodeError::Other(format!(
                "local node is not a quorum member"
            )));
        }

        Ok(())
    }

    pub fn assign_quorum_id(&mut self, quorum_type: QuorumType, members: Vec<(NodeId, PublicKey)>) {
        self.quorum_membership = Some(QuorumId::new(quorum_type, members));
    }

    fn is_harvester(&self) -> Result<()> {
        if self.quorum_type.is_none() || self.quorum_type != Some(QuorumType::Harvester) {
            return Err(NodeError::Other(format!(
                "local node is not a Harvester Node"
            )));
        }

        Ok(())
    }

    fn is_farmer(&self) -> Result<()> {
        if self.quorum_type.is_none() || self.quorum_type != Some(QuorumType::Farmer) {
            return Err(NodeError::Other(format!(
                "local node is not a Harvester Node"
            )));
        }

        Ok(())
    }

    // TODO: Refactor without use of quorum public key
    // pub fn validate_votes(
    //     &mut self,
    //     votes: Vec<Vote>,
    //     quorum_threshold: FarmerQuorumThreshold,
    //     accounts_state: &HashMap<Address, Account>,
    // ) {
    //     for vote in votes.iter() {
    //         self.validate_vote(vote.clone(), quorum_threshold, accounts_state);
    //     }
    // }

    // // The above code is handling an event of type `Vote` in a Rust
    // // program. It checks the integrity of the vote by
    // // verifying that it comes from the actual voter and prevents
    // // double voting. It then adds the vote to a pool of votes for the
    // // corresponding transaction and farmer quorum key. If
    // // the number of votes in the pool reaches the farmer
    // // quorum threshold, it sends a job to certify the transaction
    // // using the provided signature provider.
    // pub fn validate_vote(
    //     &mut self,
    //     vote: Vote,
    //     farmer_quorum_threshold: FarmerQuorumThreshold,
    //     accounts_state: &HashMap<Address, Account>,
    // ) -> Result<()> {
    //     // TODO: Harvester quorum nodes should check the integrity of the vote by verifying the vote does
    //     // come from the alleged voter Node.

    //     let sig_provider = self.sig_provider.clone();

    //     let farmer_quorum_key = hex::encode(vote.quorum_public_key.clone());
    //     let key = (vote.txn.id(), farmer_quorum_key.clone());

    //     let mut votes = self.votes_pool.get_mut(&key);

    //     if let Some(mut votes) = self.votes_pool.get_mut(&key) {
    //         if self.certified_txns_filter.contains(&key) {
    //             return Err(NodeError::Other(
    //                 "Transaction was already certified by a Harvester Node".into(),
    //             ));
    //         }

    //         votes.push(vote.clone());

    //         if votes.len() < farmer_quorum_threshold {
    //             return Err(NodeError::Other(format!(
    //                 "Not enough votes to certify transaction {}",
    //                 vote.txn.id()
    //             )));
    //         }

    //         let vote_shares = Self::group_votes_by_validity(&votes);

    //         // NOTE: revalidate the transaction because Harvesters cant trust Farmers
    //         let validated = self
    //             .validate_transactions(accounts_state, vec![vote.txn.clone()])
    //             .iter()
    //             .any(|(validated_txn, _)| validated_txn.id() == vote.txn.id());

    //         if validated {
    //             self.handle_validated_vote(
    //                 &sig_provider,
    //                 &vote_shares,
    //                 &vote.txn,
    //                 farmer_quorum_threshold,
    //                 &farmer_quorum_key,
    //             );
    //         } else {
    //             error!("Penalize Farmer for wrong votes by sending Wrong Vote event to CR Quorum");
    //         }
    //     } else {
    //         self.votes_pool.insert(key, vec![vote]);
    //     }

    //     Ok(())
    // }

    fn group_votes_by_validity(votes: &[Vote]) -> HashMap<bool, BTreeMap<NodeId, Signature>> {
        let mut vote_shares: HashMap<bool, BTreeMap<NodeId, Signature>> = HashMap::new();

        for v in votes.iter() {
            vote_shares
                .entry(v.is_txn_valid)
                .or_insert_with(BTreeMap::new)
                .insert(v.farmer_node_id.clone(), v.signature.clone());
        }

        vote_shares
    }

    // TODO: fix this fn to accept a txn list and to use the proper types
    async fn handle_validated_vote(
        &self,
        sig_provider: &SignatureProvider,
        vote_shares: &HashMap<bool, BTreeMap<NodeId, Vec<u8>>>,
        txn: &TransactionKind,
        farmer_quorum_threshold: FarmerQuorumThreshold,
        farmer_quorum_key: &str,
    ) -> Result<()> {
        let (is_txn_valid, votes_map) = vote_shares
            .iter()
            .max_by_key(|(_, votes_map)| votes_map.len())
            .map(|(key, votes_map)| (*key, votes_map.clone()))
            .ok_or(NodeError::Other("failed to get peer votes".into()))?;

        let threshold_signature = sig_provider
            .generate_quorum_signature(farmer_quorum_threshold as u16, votes_map.clone())
            .map_err(|err| {
                NodeError::Other(format!(
                    "failed to generate a quorum threshold signature: {err}"
                ))
            })?;

        todo!()
        // Ok((
        //     // votes.clone(),
        //     threshold_signature,
        //     txn.id(),
        //     farmer_quorum_key.to_string(),
        //     // txn.farmer_id.clone(),
        //     Box::new(txn.clone()),
        //     is_txn_valid,
        // ))
    }

    // TODO: fix this fn to accept a txn list and to use the proper types

    //
    // NEED ATTENTION BELOW
    // ======================================================================>
    //

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
    // TODO: revisit
    pub fn update_txn_status(&mut self, _txn_id: TransactionDigest, _status: TxnStatus) {
        // let txn_record_opt = self.tx_mempool.get(&txn_id);
        // if let Some(mut txn_record) = txn_record_opt {
        //     txn_record.status = status;
        //     self.remove_txn(txn_id);
        //     self.insert_txn(txn_record.txn);
        // }
    }
}
