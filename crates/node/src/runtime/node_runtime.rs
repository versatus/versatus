use crate::{
    consensus::{ConsensusModule, ConsensusModuleConfig},
    result::{NodeError, Result},
    state_manager::{StateManager, StateManagerConfig},
};

use block::{
    header::BlockHeader, Block, Certificate, ClaimHash, ConvergenceBlock, GenesisBlock,
    GenesisReceiver, GenesisRewards, ProposalBlock, RefHash,
};
use bulldag::graph::BullDag;
use events::{EventPublisher, Vote};
use mempool::{LeftRightMempool, MempoolReadHandleFactory, TxnRecord};
use miner::{Miner, MinerConfig};
use primitives::{Address, Epoch, NodeId, NodeType, PublicKey, QuorumKind, Round, Signature};
use ritelinked::LinkedHashMap;
use signer::engine::{QuorumMembers as InaugaratedMembers, SignerEngine};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use storage::vrrbdb::{StateStoreReadHandleFactory, VrrbDbConfig, VrrbDbReadHandle};
use theater::{ActorId, ActorState};
use tokio::task::JoinHandle;
use utils::payload::digest_data_to_bytes;
use vrrb_config::{NodeConfig, QuorumMembershipConfig};
use vrrb_core::{
    account::{Account, UpdateArgs},
    claim::Claim,
    transactions::{TransactionDigest, TransactionKind},
};

use secp256k1::{hashes::Hash, Message};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct NodeRuntime {
    // TODO: reduce scope visibility of these
    pub id: ActorId,
    pub status: ActorState,
    // TODO: make private
    pub config: NodeConfig,
    pub events_tx: EventPublisher,
    pub state_driver: StateManager,
    pub consensus_driver: ConsensusModule,
    pub mining_driver: Miner,
    pub claim: Claim,
    pub pending_quorum: Option<InaugaratedMembers>,
}

impl NodeRuntime {
    pub async fn new(
        config: &NodeConfig,
        events_tx: EventPublisher,
    ) -> std::result::Result<Self, anyhow::Error> {
        let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

        let miner_public_key = config.keypair.get_miner_public_key().to_owned();

        let signature = Claim::signature_for_valid_claim(
            miner_public_key,
            config.public_ip_address,
            config
                .keypair
                .get_miner_secret_key()
                .secret_bytes()
                .to_vec(),
        )?;

        let claim = Claim::new(
            miner_public_key,
            Address::new(miner_public_key),
            config.public_ip_address,
            signature,
            config.id.clone(),
        )
        .map_err(NodeError::from)?;

        let mut vrrbdb_config = VrrbDbConfig::default();

        if config.db_path() != &vrrbdb_config.path {
            vrrbdb_config.with_path(config.db_path().to_path_buf());
        }

        let database = storage::vrrbdb::VrrbDb::new(vrrbdb_config);
        let mempool = LeftRightMempool::new();

        let state_driver = StateManager::new(StateManagerConfig {
            database: database.clone(),
            mempool,
            dag: dag.clone(),
            claim: claim.clone(),
        });

        let (_, miner_secret_key) = config.keypair.get_secret_keys();
        let (_, miner_public_key) = config.keypair.get_public_keys();

        let miner_config = MinerConfig {
            secret_key: *miner_secret_key,
            public_key: *miner_public_key,
            ip_address: config.public_ip_address,
            dag: dag.clone(),
            claim: claim.clone(),
        };

        let miner = miner::Miner::new(miner_config, config.id.clone()).map_err(NodeError::from)?;
        let consensus_driver = ConsensusModule::new(
            ConsensusModuleConfig {
                keypair: config.keypair.clone(),
                node_config: config.clone(),
                validator_public_key: config.keypair.validator_public_key_owned(),
            },
            state_driver.mempool_read_handle_factory(),
            database.state_store_factory(),
            database.claim_store_factory(),
            // TODO: Replace with a configurable number
            10,
        )?;

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ActorState::Stopped,
            config: config.to_owned(),
            events_tx,
            state_driver,
            consensus_driver,
            mining_driver: miner,
            claim,
            pending_quorum: None,
        })
    }

    pub fn certified_convergence_block_exists_within_dag(&self, block_hash: String) -> bool {
        if let Ok(guard) = self.state_driver.dag.read() {
            if let Some(vertex) = guard.get_vertex(block_hash) {
                if let Block::Convergence { block } = vertex.get_data() {
                    return block.certificate.is_some();
                } else {
                    return false;
                }
            }
        }
        false
    }

    pub fn certified_genesis_block_exists_within_dag(&self, block_hash: String) -> bool {
        if let Ok(guard) = self.state_driver.dag.read() {
            if let Some(vertex) = guard.get_vertex(block_hash) {
                if let Block::Genesis { block } = vertex.get_data() {
                    return block.certificate.is_some();
                } else {
                    return false;
                }
            }
        }
        false
    }

    pub fn config_ref(&self) -> &NodeConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut NodeConfig {
        &mut self.config
    }

    pub fn config_owned(&self) -> NodeConfig {
        self.config.clone()
    }

    fn _setup_reputation_module(
    ) -> std::result::Result<Option<JoinHandle<Result<()>>>, anyhow::Error> {
        Ok(None)
    }

    fn _setup_credit_model_module(
    ) -> std::result::Result<Option<JoinHandle<Result<()>>>, anyhow::Error> {
        Ok(None)
    }

    pub fn has_required_node_type(&self, intended_node_type: NodeType, action: &str) -> Result<()> {
        if self.config.node_type != intended_node_type {
            return Err(NodeError::Other(format!(
                "Only {intended_node_type} nodes are allowed to: {action}"
            )));
        }
        Ok(())
    }

    pub fn belongs_to_correct_quorum(
        &self,
        intended_quorum: QuorumKind,
        action: &str,
    ) -> Result<()> {
        if let Some(membership) = self.quorum_membership() {
            let quorum_kind = membership.quorum_kind();

            if quorum_kind != intended_quorum {
                return Err(NodeError::Other(format!(
                    "Only {intended_quorum} nodes are allowed to: {action}"
                )));
            }
        } else {
            return Err(NodeError::Other(
                "No quorum configuration found for node".to_string(),
            ));
        }

        Ok(())
    }

    pub fn quorum_membership(&self) -> Option<QuorumMembershipConfig> {
        self.consensus_driver
            .quorum_driver
            .membership_config
            .clone()
    }

    pub fn state_read_handle(&self) -> VrrbDbReadHandle {
        self.state_driver.read_handle()
    }

    pub fn state_store_read_handle_factory(&self) -> StateStoreReadHandleFactory {
        self.state_driver.database.state_store_factory()
    }

    pub fn mempool_read_handle_factory(&self) -> MempoolReadHandleFactory {
        self.state_driver.mempool_read_handle_factory()
    }

    pub fn mempool_snapshot(&self) -> HashMap<TransactionDigest, TxnRecord> {
        self.mempool_read_handle_factory().entries()
    }

    // TODO: This should be a const function
    pub fn distribute_genesis_reward(
        &self,
        receivers: Vec<GenesisReceiver>,
    ) -> Result<GenesisRewards> {
        self.has_required_node_type(NodeType::Miner, "produce genesis transactions")?;
        Ok(GenesisRewards(
            receivers.iter().map(|rc| (rc.to_owned(), 10000)).collect(),
        ))
    }

    pub fn mine_genesis_block(&self, genesis_rewards: GenesisRewards) -> Result<GenesisBlock> {
        self.has_required_node_type(NodeType::Miner, "mine genesis block")?;

        let claim = self.state_driver.dag.claim();

        let claim_list = vec![(claim.hash, claim.clone())];

        let claim_list_hash = digest_data_to_bytes(&claim_list);
        let seed = 0;
        let round = 0;
        let epoch = 0;

        let header = BlockHeader::genesis(
            seed,
            round,
            epoch,
            claim.clone(),
            self.config.keypair.miner_secret_key_owned(),
            hex::encode(claim_list_hash),
        );

        let block_header = header.clone();
        let block_hash = digest_data_to_bytes(&(
            header.ref_hashes,
            header.round,
            header.block_seed,
            header.next_block_seed,
            header.block_height,
            header.timestamp,
            header.txn_hash,
            header.miner_claim,
            header.claim_list_hash,
            header.block_reward,
            header.next_block_reward,
            header.miner_signature,
        ));

        let mut claims = LinkedHashMap::new();
        claims.insert(claim.hash, claim);

        let genesis = GenesisBlock {
            header: block_header,
            genesis_rewards,
            claims,
            hash: hex::encode(block_hash),
            certificate: None,
        };

        Ok(genesis)
    }

    pub fn verify_genesis_block_origin(&self, genesis_block: GenesisBlock) -> Result<()> {
        let miner_signature = genesis_block.header.miner_signature;
        let miner_id = genesis_block.header.miner_claim.node_id.clone();
        let hashed = self.hash_block_header(&genesis_block.header);
        let message = Message::from(hashed);

        self.consensus_driver
            .verify_signature(&miner_id, &miner_signature, &message)?;

        Ok(())
    }

    fn hash_block_header(&self, header: &BlockHeader) -> secp256k1::hashes::sha256::Hash {
        let hashed = format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            header.ref_hashes,
            header.round,
            header.epoch,
            header.block_seed,
            header.next_block_seed,
            header.block_height,
            header.timestamp,
            header.txn_hash,
            header.miner_claim,
            header.claim_list_hash,
            header.block_reward,
            header.next_block_reward,
        );

        secp256k1::hashes::sha256::Hash::hash(hashed.as_bytes())
    }

    // TODO: simplify logic to be under handle_harvester_signature_received
    pub fn certify_genesis_block(
        &mut self,
        genesis: GenesisBlock,
        node_id: String,
        sig: Signature,
    ) -> Result<Certificate> {
        self.consensus_driver.is_harvester()?;
        self.consensus_driver
            .sig_engine
            .verify(&node_id, &sig, &genesis.hash)
            .map_err(|err| NodeError::Other(err.to_string()))?;
        let set = self
            .state_driver
            .dag
            .add_signer_to_block(
                genesis.hash.clone(),
                sig,
                node_id,
                &self.consensus_driver.sig_engine,
            )
            .map_err(|err| NodeError::Other(err.to_string()))?;
        let certificate = self
            .consensus_driver
            .certify_genesis_block(genesis, set.into_iter().collect())?;

        Ok(certificate)
    }

    pub fn mine_proposal_block(
        &mut self,
        ref_hash: RefHash,
        claim_map: HashMap<String, Claim>,
        round: Round,
        epoch: Epoch,
        from: Claim,
        sig_engine: SignerEngine,
    ) -> Result<ProposalBlock> {
        self.consensus_driver.is_harvester()?;
        let txns = self
            .consensus_driver
            .quorum_certified_txns
            .iter()
            .take(PULL_TXN_BATCH_SIZE);

        // NOTE: Read updated claims
        // let claim_map = self.vrrbdb_read_handle.claim_store_values();
        let claim_list = claim_map
            .values()
            .map(|from| (from.hash, from.clone()))
            .collect();

        //TODO: variable _cert is not being used.
        let txns_list: LinkedHashMap<TransactionDigest, TransactionKind> = txns
            .into_iter()
            .map(|(digest, (txn, _cert))| {
                //                if let Err(err) = self
                //                    .consensus_driver
                //                    .certified_txns_filter
                //                    .push(&txn.id().to_string())
                //                {
                //                    telemetry::error!("Error pushing txn to certified txns filter: {err}");
                //                }
                (digest.clone(), txn.clone())
            })
            .collect();

        Ok(ProposalBlock::build(
            ref_hash, round, epoch, txns_list, claim_list, from, sig_engine,
        ))
    }

    pub fn mine_convergence_block(&mut self) -> Result<ConvergenceBlock> {
        self.has_required_node_type(NodeType::Miner, "mine convergence block")?;
        self.mining_driver
            .mine_convergence_block()
            .ok_or(NodeError::Other(
                "Could not mine convergence block".to_string(),
            ))
    }

    pub fn certify_convergence_block(&mut self, block: ConvergenceBlock) -> Result<()> {
        self.consensus_driver.is_harvester()?;
        let last_block_header =
            self.state_driver
                .dag
                .last_confirmed_block_header()
                .ok_or(NodeError::Other(format!(
                    "Node {} does not have a last confirmed block header",
                    self.config.id
                )))?;

        let next_txn_trie_hash = self.state_driver.transactions_root_hash()?;
        let certs = self
            .state_driver
            .dag
            .check_certificate_threshold_reached(&block.hash, &self.consensus_driver.sig_engine)?;

        self.consensus_driver.certify_convergence_block(
            block,
            last_block_header,
            next_txn_trie_hash.clone(),
            self.mining_driver.clone(),
            self.state_driver.dag.dag().clone(),
            certs.into_iter().collect(),
        )?;

        Ok(())
    }

    pub fn transactions_root_hash(&self) -> Result<String> {
        self.state_driver.transactions_root_hash()
    }

    pub fn state_root_hash(&self) -> Result<String> {
        self.state_driver.state_root_hash()
    }

    pub fn state_snapshot(&self) -> HashMap<Address, Account> {
        let handle = self.state_driver.read_handle();
        handle.state_store_values()
    }

    pub fn transactions_snapshot(&self) -> HashMap<TransactionDigest, TransactionKind> {
        let handle = self.state_driver.read_handle();
        handle.transaction_store_values()
    }

    pub fn claims_snapshot(&self) -> HashMap<NodeId, Claim> {
        let handle = self.state_driver.read_handle();
        handle.claim_store_values()
    }

    async fn _get_transaction_by_id(
        &self,
        _transaction_digest: TransactionDigest,
    ) -> Result<TransactionKind> {
        todo!()
    }

    pub fn create_account(&mut self, public_key: PublicKey) -> Result<Address> {
        let account = Account::new(public_key.into());

        self.state_driver
            .insert_account(public_key.into(), account)?;

        Ok(public_key.into())
    }

    pub fn update_account(&mut self, args: UpdateArgs) -> Result<()> {
        self.state_driver.update_account(args)
    }

    pub fn get_account_by_address(&self, address: &Address) -> Result<Account> {
        self.state_driver.get_account(address)
    }

    pub fn get_round(&self) -> Result<Round> {
        let header =
            self.state_driver
                .dag
                .last_confirmed_block_header()
                .ok_or(NodeError::Other(
                    "failed to fetch latest block header from dag".to_string(),
                ))?;

        Ok(header.round)
    }

    pub fn get_claims_by_account_address(&self, address: &Address) -> Result<Vec<Claim>> {
        self.state_driver.get_claims_by_account_address(address)
    }

    pub fn get_claim_hashes(&self) -> Result<Vec<ClaimHash>> {
        todo!()
    }

    pub fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Vec<Claim>> {
        self.state_driver.get_claims(claim_hashes)
    }

    pub fn insert_txn_to_mempool(&mut self, txn: TransactionKind) -> Result<TransactionDigest> {
        self.state_driver.insert_txn_to_mempool(txn)
    }

    pub fn extend_mempool(&mut self, txns: &[TransactionKind]) -> Result<()> {
        self.state_driver.extend_mempool(txns)
    }

    pub fn memmpol_len(&self) -> usize {
        self.state_driver.mempool_len()
    }

    pub fn validate_transaction_kind(
        &mut self,
        digest: TransactionDigest,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
    ) -> Result<(TransactionKind, bool)> {
        self.has_required_node_type(NodeType::Validator, "validate transactions")?;
        self.belongs_to_correct_quorum(QuorumKind::Farmer, "validate transactions")?;
        let validated_transaction_kind =
            self.consensus_driver
                .validate_transaction_kind(&digest, mempool_reader, state_reader);

        match validated_transaction_kind {
            Ok(transaction_kind) => Ok((transaction_kind, true)),
            Err(_) => {
                let handle = self.mempool_read_handle_factory().handle();
                let transaction_record = handle.get(&digest);
                match transaction_record {
                    Some(record) => Ok((record.txn.clone(), false)),
                    None => Err(NodeError::Other("transaction record not found".to_string())),
                }
            },
        }
    }

    pub fn cast_vote_on_transaction_kind(
        &mut self,
        transaction: TransactionKind,
        validity: bool,
    ) -> Result<Vote> {
        self.consensus_driver
            .cast_vote_on_transaction_kind(transaction, validity)
    }
}
