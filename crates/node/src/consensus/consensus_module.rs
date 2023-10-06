use super::{QuorumModule, QuorumModuleConfig};
use crate::{NodeError, Result};
use block::{
    header::BlockHeader, Block, Certificate, ConvergenceBlock, GenesisBlock, ProposalBlock,
};
use bulldag::graph::BullDag;
use events::{SyncPeerData, Vote};
use mempool::MempoolReadHandleFactory;
use miner::conflict_resolver::Resolver;
use primitives::{
    NodeId, NodeTypeBytes, PKShareBytes, PayloadBytes, PublicKey, QuorumId, QuorumKind,
    QuorumPublicKey, RawSignature, Signature, ValidatorPublicKey,
};
use serde::{Deserialize, Serialize};
use signer::engine::SignerEngine;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};
use storage::vrrbdb::{ClaimStoreReadHandleFactory, StateStoreReadHandleFactory};
use validator::txn_validator::TxnValidatorError;
use validator::validator_core_manager::ValidatorCoreManager;
use vrrb_config::{NodeConfig, QuorumMembershipConfig};
use vrrb_core::transactions::{
    QuorumCertifiedTxn, Transaction, TransactionDigest, TransactionKind,
};
use vrrb_core::{bloom::Bloom, keypair::Keypair};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

// TODO: Move this to primitives

#[derive(Debug)]
pub struct ConsensusModuleConfig {
    pub keypair: Keypair,
    pub node_config: NodeConfig,
    // pub dkg_generator: DkgEngine,
    pub validator_public_key: PublicKey,
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
    pub(crate) quorum_kind: Option<QuorumKind>,
    pub votes_pool: HashMap<QuorumId, HashMap<TransactionDigest, HashSet<Vote>>>,
    pub(crate) validator_core_manager: ValidatorCoreManager,
}

impl ConsensusModule {
    pub fn new(
        cfg: ConsensusModuleConfig,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
        claim_reader: ClaimStoreReadHandleFactory,
        cores: usize,
    ) -> Result<Self> {
        let quorum_module_config = QuorumModuleConfig {
            membership_config: None,
            node_config: cfg.node_config.clone(),
        };

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
            quorum_kind: None,
            validator_core_manager,
            votes_pool: Default::default(),
        })
    }

    pub fn validator_public_key_owned(&self) -> PublicKey {
        self.keypair.validator_public_key_owned()
    }

    pub fn certify_block(
        &mut self,
        block: Block,
        _last_block_header: BlockHeader,
        prev_txn_root_hash: String,
        //        next_txn_root_hash: String,
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

        self.sig_engine
            .verify_batch(&certs, &block.hash())
            .map_err(|err| NodeError::Other(err.to_string()))?;

        //TODO: If Quorums are pending inauguration include inauguration info
        let certificate = Certificate {
            signatures: certs.clone(),
            inauguration: None,
            root_hash: prev_txn_root_hash,
            block_hash,
        };

        Ok(certificate)
    }

    pub fn certify_genesis_block(
        &mut self,
        block: GenesisBlock,
        certs: Vec<(NodeId, Signature)>,
    ) -> Result<Certificate> {
        let txn_trie_hash = block.header.txn_hash.clone();
        let last_block_header = block.header.clone();

        self.certify_block(
            block.into(),
            last_block_header,
            txn_trie_hash.clone(),
            certs,
        )
    }

    pub fn certify_convergence_block<R: Resolver<Proposal = ProposalBlock>>(
        &mut self,
        block: ConvergenceBlock,
        last_block_header: BlockHeader,
        _next_txn_root_hash: String,
        resolver: R,
        dag: Arc<RwLock<BullDag<Block, String>>>,
        certs: Vec<(NodeId, Signature)>,
    ) -> Result<Certificate> {
        let prev_txn_root_hash = last_block_header.txn_hash.clone();

        self.precheck_convergence_block(
            block.clone(),
            last_block_header.clone(),
            resolver,
            dag.clone(),
        )?;
        self.certify_block(block.into(), last_block_header, prev_txn_root_hash, certs)
    }

    async fn sign_convergence_block(
        &mut self,
        block: ConvergenceBlock,
    ) -> Result<(String, PublicKey, Signature)> {
        let block_hash_bytes = hex::decode(block.hash.clone())
            .map_err(|err| NodeError::Other(format!("unable to decode block hash: {err}")))?;

        let signature = self.sig_engine.sign(block_hash_bytes).map_err(|err| {
            NodeError::Other(format!("failed to generate partial signature: {err}"))
        })?;

        Ok((
            block.hash.clone(),
            self.sig_engine.public_key(),
            signature.clone(),
        ))
    }

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

    pub fn validate_transaction_kind(
        &mut self,
        digest: &TransactionDigest,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
    ) -> validator::txn_validator::Result<TransactionKind> {
        self.is_farmer()
            .map_err(|err| TxnValidatorError::Other(err.to_string()))?;
        self.validator_core_manager
            .validate_transaction_kind(digest, mempool_reader, state_reader)
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
        self.is_farmer()?;

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

        if let Some((quorum_id, QuorumKind::Farmer)) = self.get_node_quorum_id(&vote.farmer_id) {
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

    fn get_node_quorum_id(&self, node_id: &NodeId) -> Option<(QuorumId, QuorumKind)> {
        let quorum_members = self.sig_engine.quorum_members();
        for (quorum_id, quorum_data) in quorum_members.0.iter() {
            if quorum_data.members.contains_key(node_id) {
                return Some((quorum_id.clone(), quorum_data.quorum_kind.clone()));
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

    pub fn assign_quorum_id(&mut self, quorum_kind: QuorumKind, members: Vec<(NodeId, PublicKey)>) {
        self.quorum_membership = Some(QuorumId::new(quorum_kind, members));
    }

    pub(crate) fn is_harvester(&self) -> Result<()> {
        if self.quorum_kind.is_none() || self.quorum_kind != Some(QuorumKind::Harvester) {
            return Err(NodeError::Other(format!(
                "local node is not a Harvester Node"
            )));
        }

        Ok(())
    }

    pub(crate) fn is_farmer(&self) -> Result<()> {
        if self.quorum_kind.is_none() || self.quorum_kind != Some(QuorumKind::Farmer) {
            return Err(NodeError::Other(format!("local node is not a Farmer Node")));
        }

        Ok(())
    }

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
}
