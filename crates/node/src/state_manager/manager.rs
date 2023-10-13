use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, Certificate, ClaimHash, ConvergenceBlock, ProposalBlock};
use bulldag::{
    graph::{BullDag, GraphError},
    vertex::Vertex,
};
use ethereum_types::U256;
use events::Event;
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::{Address, NodeId, Round};
use signer::engine::{QuorumMembers, SignerEngine};
use storage::vrrbdb::{types::*, ApplyBlockResult};
use storage::{
    storage_utils::StorageError,
    vrrbdb::{Claims, VrrbDb, VrrbDbReadHandle},
};
use telemetry::info;
use theater::{ActorId, ActorState};
use vrrb_core::{account::Account, claim::Claim};
use vrrb_core::{
    account::UpdateArgs,
    transactions::{Transaction, TransactionDigest, TransactionKind},
};

use crate::{data_store::DataStore, state_reader::StateReader};
use crate::{NodeError, Result};

use super::{
    utils::{consolidate_update_args, get_update_args},
    DagModule, GraphResult,
};

/// Provides a convenient configuration struct for building a
/// StateManager
#[derive(Debug, Clone)]
pub struct StateManagerConfig {
    pub database: VrrbDb,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub mempool: LeftRightMempool,
    pub claim: Claim,
}

#[derive(Debug, Clone)]
pub struct StateManager {
    pub(crate) actor_id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) dag: DagModule,
    pub(crate) database: VrrbDb,
    pub(crate) mempool: LeftRightMempool,
}

impl StateManager {
    pub fn new(config: StateManagerConfig) -> Self {
        let dag_module = DagModule::new(config.dag.clone(), config.claim.clone());

        Self {
            actor_id: uuid::Uuid::new_v4().to_string(),
            database: config.database,
            status: ActorState::Stopped,
            dag: dag_module,
            mempool: config.mempool,
        }
    }

    pub fn append_convergence(
        &mut self,
        convergence: &ConvergenceBlock,
    ) -> GraphResult<ApplyBlockResult> {
        let opt = self.dag.append_convergence(convergence)?;
        if let Some(cblock) = opt {
            let ref_blocks = self.dag.get_convergence_reference_blocks(convergence);
            let proposals: Vec<ProposalBlock> = ref_blocks
                .iter()
                .filter_map(|vertex| match vertex.get_data() {
                    Block::Proposal { block } => Some(block.clone()),
                    _ => None,
                })
                .collect();

            let res = self.apply_convergence_block(&cblock, &proposals)?;
            return Ok(res);
        }

        Err(GraphError::Other(
            "unable to append and apply convergence block".to_string(),
        ))
    }

    pub fn apply_convergence_block(
        &mut self,
        convergence: &ConvergenceBlock,
        proposals: &[ProposalBlock],
    ) -> GraphResult<ApplyBlockResult> {
        let res = self
            .database
            .apply_convergence_block(convergence, proposals)
            .map_err(|err| GraphError::Other(err.to_string()))?;
        Ok(res)
    }

    pub fn append_certificate_to_convergence_block(
        &mut self,
        certificate: &Certificate,
    ) -> GraphResult<Option<ConvergenceBlock>> {
        self.dag
            .append_certificate_to_convergence_block(certificate)
    }

    pub fn export_state(&self) {
        self.database.export_state();
    }

    /// Produces the read handle for the VrrbDb instance in this
    /// struct. VrrbDbReadHandle provides a ReadHandleFactory for
    /// each of the StateStore, TransactionStore and ClaimStore.
    pub fn read_handle(&self) -> VrrbDbReadHandle {
        self.database.read_handle()
    }

    pub fn mempool_read_handle_factory(&self) -> MempoolReadHandleFactory {
        self.mempool.factory()
    }

    pub fn transactions_root_hash(&self) -> Result<String> {
        let root_hash = self.database.transactions_root_hash()?;
        let root_hash_hex = hex::encode(root_hash.0);
        Ok(root_hash_hex)
    }

    pub fn state_root_hash(&self) -> Result<String> {
        let root_hash = self.database.state_root_hash()?;
        let root_hash_hex = hex::encode(root_hash.0);
        Ok(root_hash_hex)
    }

    pub fn claims_root_hash(&self) -> Result<String> {
        let root_hash = self.database.claims_root_hash()?;
        let root_hash_hex = hex::encode(root_hash.0);
        Ok(root_hash_hex)
    }

    //TODO: Move to test configured trait
    pub fn write_vertex(&mut self, vertex: &Vertex<Block, BlockHash>) -> Result<()> {
        self.dag
            .write_vertex(vertex)
            .map_err(|err| NodeError::Other(format!("{:?}", err)))
    }

    /// Inserts a Transaction into the TransactionStore and
    /// emits an event to inform other modules that a Transaction
    /// has been added to the TransactionStore.
    // This is unneccessary under the system architecture, btw.
    pub(crate) async fn confirm_txn(&mut self, txn: TransactionKind) -> Result<TransactionDigest> {
        let txn_hash = txn.id();

        info!("Storing transaction {txn_hash} in confirmed transaction store");

        //TODO: call checked methods instead
        self.database.insert_transaction(txn)?;

        Ok(txn_hash)
    }

    pub fn commit(&mut self) {
        self.database.commit_state();
    }

    /// Given the hash of a `ConvergenceBlock` this method
    /// updates the StateStore, ClaimStore and TransactionStore
    /// for all new claims and transactions (excluding
    /// ClaimStaking transactions currently).
    pub fn update_state(&mut self, block_hash: BlockHash) -> Result<()> {
        if let Some(mut round_blocks) = self.get_proposal_blocks(block_hash) {
            let update_list = self.get_update_list(&mut round_blocks);
            let update_args = get_update_args(update_list);
            let consolidated_update_args = consolidate_update_args(update_args);
            consolidated_update_args.into_iter().for_each(|(_, args)| {
                if let Err(err) = self.database.update_account(args) {
                    telemetry::error!("error updating account: {err}");
                }
            });

            let proposals = round_blocks.proposals.clone();

            self.update_txn_trie(&proposals);
            self.update_claim_store(&proposals);

            return Ok(());
        }

        Err(NodeError::Other(
            "Convergene block not found in DAG".to_string(),
        ))
    }

    /// Provided a reference to an array of `ProposalBlock`s
    /// making up the current round's `ConvergenceBlock`, writes all
    /// the conflict resolved transactions into the `TransactionTrie`
    fn update_txn_trie(&mut self, proposals: &[ProposalBlock]) {
        let consolidated: HashSet<TransactionKind> = {
            let nested: Vec<HashSet<TransactionKind>> = proposals
                .iter()
                .map(|block| block.txns.iter().map(|(_, v)| v.clone()).collect())
                .collect();

            nested.into_iter().flatten().collect()
        };

        self.database
            .extend_transactions(consolidated.into_iter().collect());
    }

    /// Provided a reference to an array of `ProposalBlock`s
    /// making up the current round's `ConvergenceBlock`, writes
    /// all the new, conflict resolved, claims into the `ClaimStore`
    fn update_claim_store(&mut self, proposals: &[ProposalBlock]) {
        let consolidated: HashSet<(U256, Option<Claim>)> = {
            let nested: Vec<HashSet<(U256, Option<Claim>)>> = {
                proposals
                    .iter()
                    .map(|block| {
                        block
                            .claims
                            .iter()
                            .map(|(k, v)| (*k, Some(v.clone())))
                            .collect()
                    })
                    .collect()
            };

            nested.into_iter().flatten().collect()
        };

        self.database
            .extend_claims(consolidated.into_iter().collect());
    }

    /// Provides a method to convert a `RoundBlocks` wrapper struct into
    /// a HashSet of unique `StateUpdate`s
    fn get_update_list(&self, round_blocks: &mut RoundBlocks) -> HashSet<StateUpdate> {
        let convergence = round_blocks.convergence.clone();
        let filtered_proposals: Vec<ProposalBlock> = round_blocks
            .proposals
            .iter_mut()
            .map(|block| {
                if let Some(digests) = convergence.txns.get(&block.hash) {
                    block.txns.retain(|digest, _| digests.contains(digest))
                }
                block.clone()
            })
            .collect();

        let mut updates: HashSet<StateUpdate> = HashSet::new();

        filtered_proposals.iter().for_each(|block| {
            let subset = HashSet::from_block(block.clone());
            updates.extend(subset);
        });

        updates
    }

    /// Inserts an account into the `VrrbDb` `StateStore`. This method Should
    /// only be used for *new* accounts
    pub fn insert_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.database
            .insert_account(key, account)
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    pub fn extend_accounts(&mut self, accounts: Vec<(Address, Option<Account>)>) -> Result<()> {
        self.database.extend_accounts(accounts);
        Ok(())
    }

    /// Enters into the DAG and collects and returns the current round
    /// `ConvergenceBlock` and all its source `ProposalBlock`s
    fn get_proposal_blocks(&self, index: BlockHash) -> Option<RoundBlocks> {
        let guard_result = self.dag.read();

        if let Ok(guard) = guard_result {
            let vertex_option = guard.get_vertex(index);
            match &vertex_option {
                Some(vertex) => {
                    if let Block::Convergence { block } = vertex.get_data() {
                        let proposals = self.convert_sources(self.get_sources(vertex));

                        return Some(RoundBlocks {
                            convergence: block,
                            proposals,
                        });
                    }
                },
                None => {},
            }
        }

        None
    }

    /// Enters into the DAG and gets all the sources of a given vertex
    /// this is used primarily to capture all the `ProposalBlock`s
    /// that make up the current round `ConvergenceBlock`
    fn get_sources(&self, vertex: &Vertex<Block, BlockHash>) -> Vec<Vertex<Block, BlockHash>> {
        let mut source_vertices = Vec::new();
        let guard_result = self.dag.read();
        if let Ok(guard) = guard_result {
            let sources = vertex.get_sources();
            sources.iter().for_each(|index| {
                let source_option = guard.get_vertex(index.to_string());
                if let Some(source) = source_option {
                    source_vertices.push(source.clone());
                }
            });
        }

        source_vertices
    }

    /// Converts the discovered `source` `Vertex`s into a Vector of
    /// `ProposalBlock`s, filtering any other types of blocks out.
    fn convert_sources(&self, sources: Vec<Vertex<Block, BlockHash>>) -> Vec<ProposalBlock> {
        let blocks: Vec<Block> = sources.iter().map(|vtx| vtx.get_data()).collect();

        let mut proposals = Vec::new();

        blocks.iter().for_each(|block| {
            if let Block::Proposal { block } = &block {
                proposals.push(block.clone())
            }
        });

        proposals
    }

    pub fn handle_block_received(
        &mut self,
        block: &mut Block,
        sig_engine: SignerEngine,
    ) -> Result<Event> {
        match block {
            Block::Genesis { ref mut block } => {
                if let Err(e) = self.dag.append_genesis(&block) {
                    let err_note = format!("Encountered GraphError: {e:?}");
                    return Err(NodeError::Other(err_note));
                };
            },
            Block::Proposal { ref mut block } => {
                if let Err(e) = self.dag.append_proposal(&block, sig_engine.clone()) {
                    let err_note = format!("Encountered GraphError: {e:?}");
                    return Err(NodeError::Other(err_note));
                }
            },
            Block::Convergence { ref mut block } => {
                if let Err(e) = self.dag.append_convergence(block) {
                    let err_note = format!("Encountered GraphError: {e:?}");
                    return Err(NodeError::Other(err_note));
                }

                if block.certificate.is_none() {
                    if let Some(header) = self.dag.last_confirmed_block_header() {
                        let event = Event::ConvergenceBlockPrecheckRequested {
                            convergence_block: block.clone(),
                            block_header: header,
                        };

                        return Ok(event);
                    }
                }
            },
        }

        Ok(Event::BlockAppended(block.hash()))
    }

    pub fn apply_block(&mut self, block: Block) -> Result<ApplyBlockResult> {
        let apply_result = self
            .database
            .apply_block(block)
            .map_err(|err| NodeError::Other(err.to_string()))?;

        Ok(apply_result)
    }

    pub fn insert_txn_to_mempool(&mut self, txn: TransactionKind) -> Result<TransactionDigest> {
        let txn_hash = txn.id();

        self.mempool
            .insert(txn)
            .map_err(|err| NodeError::Other(err.to_string()))?;

        Ok(txn_hash)
    }

    pub fn extend_mempool(&mut self, txns: &[TransactionKind]) -> Result<()> {
        let txn_batch = txns.iter().map(|txn| txn.to_owned()).collect();
        self.mempool
            .extend(txn_batch)
            .map_err(|err| NodeError::Other(err.to_string()))?;

        Ok(())
    }

    /// Return the number of key-value pairs in the map.
    ///
    pub fn mempool_len(&self) -> usize {
        self.mempool.len()
    }

    #[deprecated = "use insert_txn_to_mempool instead"]
    pub fn handle_new_txn_created(&mut self, txn: TransactionKind) -> Result<TransactionDigest> {
        info!("Storing transaction in mempool for validation");
        self.insert_txn_to_mempool(txn)
    }

    pub async fn handle_transaction_validated(&mut self, txn: TransactionKind) -> Result<()> {
        self.mempool
            .remove(&txn.id())
            .map_err(|err| NodeError::Other(err.to_string()))?;

        self.confirm_txn(txn).await?;

        Ok(())
    }

    pub fn handle_quorum_members_received(&mut self, quorum_members: QuorumMembers) {
        self.dag.set_quorum_members(quorum_members)
    }

    pub fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Claims> {
        Ok(self
            .database
            .claim_store_factory()
            .handle()
            .entries()
            .clone()
            .into_iter()
            .filter(|(_, claim)| claim_hashes.contains(&claim.hash))
            .map(|(_, claim)| claim)
            .collect())
    }

    pub fn get_claims_by_account_address(&self, address: &Address) -> Result<Vec<Claim>> {
        Ok(self
            .database
            .claim_store_factory()
            .handle()
            .entries()
            .clone()
            .into_iter()
            .filter(|(_, claim)| &claim.address == address)
            .map(|(_, claim)| claim)
            .collect())
    }

    pub fn update_account(&mut self, update_args: UpdateArgs) -> Result<()> {
        self.database
            .update_account(update_args)
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    pub fn get_account(&self, address: &Address) -> Result<Account> {
        let handle = self.database.state_store_factory().handle();
        handle
            .get(address)
            .map_err(|err| NodeError::Other(err.to_string()))
    }

    /// For testing purposes only. Do not use in production.
    pub fn insert_claims(&mut self, claims: Vec<Claim>) -> Result<()> {
        for claim in claims {
            self.database.insert_claim(claim)?
        }
        Ok(())
    }

    pub fn dag(&self) -> Arc<RwLock<BullDag<Block, String>>> {
        self.dag.dag().clone()
    }
}

#[async_trait::async_trait]
impl DataStore<VrrbDbReadHandle> for VrrbDb {
    type Error = StorageError;

    fn state_reader(&self) -> VrrbDbReadHandle {
        self.read_handle()
    }
}

#[async_trait::async_trait]
impl StateReader for VrrbDbReadHandle {
    /// Returns a full list of all accounts within state
    async fn state_snapshot(&self) -> Result<HashMap<Address, Account>> {
        self.state_snapshot().await
    }

    /// Returns a full list of transactions pending to be confirmed
    async fn mempool_snapshot(&self) -> Result<HashMap<TransactionDigest, TransactionKind>> {
        self.mempool_snapshot().await
    }

    /// Get a transaction from state
    async fn get_transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<TransactionKind> {
        todo!()
    }

    /// List a group of transactions
    async fn list_transactions(
        &self,
        digests: Vec<TransactionDigest>,
    ) -> Result<HashMap<TransactionDigest, TransactionKind>> {
        todo!()
    }

    async fn get_account(&self, address: Address) -> Result<Account> {
        todo!()
    }

    async fn get_round(&self) -> Result<Round> {
        todo!()
    }

    async fn get_blocks(&self) -> Result<Vec<Block>> {
        todo!()
    }

    async fn get_transaction_count(&self) -> Result<usize> {
        todo!()
    }

    async fn get_claims_by_account_id(&self) -> Result<Vec<Claim>> {
        todo!()
    }

    async fn get_claim_hashes(&self) -> Result<Vec<ClaimHash>> {
        todo!()
    }

    async fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Claims> {
        todo!()
    }

    async fn get_last_block(&self) -> Result<Block> {
        todo!()
    }

    fn state_store_values(&self) -> HashMap<Address, Account> {
        self.state_store_values()
    }

    fn transaction_store_values(&self) -> HashMap<TransactionDigest, TransactionKind> {
        self.transaction_store_values()
    }

    fn claim_store_values(&self) -> HashMap<NodeId, Claim> {
        self.claim_store_values()
    }
}
