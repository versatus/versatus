use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, Certificate, ClaimHash, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use ethereum_types::U256;
use events::{Event, EventMessage, EventPublisher, Vote};
use mempool::LeftRightMempool;
use primitives::{
    Address, NodeId, ProgramExecutionOutput, RawSignature, Round, TxnValidationStatus,
};
use storage::{
    storage_utils::StorageError,
    vrrbdb::{Claims, StateStoreReadHandle, VrrbDb, VrrbDbReadHandle},
};
use telemetry::info;
use theater::{ActorId, ActorState};
use vrrb_core::{
    account::Account,
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

use crate::{data_store::DataStore, state_manager::types::*, state_reader::StateReader};
use crate::{NodeError, Result};

use super::{
    utils::{consolidate_update_args, get_update_args},
    DagModule,
};

/// Provides a convenient configuration struct for building a
/// StateManager
#[derive(Debug, Clone)]
pub struct StateManagerConfig {
    pub database: VrrbDb,
    pub events_tx: EventPublisher,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub mempool: LeftRightMempool,
    pub claim: Claim,
}

#[derive(Debug)]
pub struct StateManager {
    pub(crate) id: ActorId,
    pub(crate) status: ActorState,
    pub(crate) events_tx: EventPublisher,
    pub(crate) dag: DagModule,
    pub(crate) database: VrrbDb,
    pub(crate) mempool: LeftRightMempool,
}

impl StateManager {
    pub fn new(config: StateManagerConfig) -> Self {
        let dag_module = DagModule::new(
            config.dag.clone(),
            config.events_tx.clone(),
            config.claim.clone(),
        );

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            database: config.database,
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            dag: dag_module,
            mempool: config.mempool,
        }
    }

    pub fn _export_state(&self) {
        self.database.export_state();
    }

    /// Produces the read handle for the VrrbDb instance in this
    /// struct. VrrbDbReadHandle provides a ReadHandleFactory for
    /// each of the StateStore, TransactionStore and ClaimStore.
    pub fn _read_handle(&self) -> VrrbDbReadHandle {
        self.database.read_handle()
    }

    /// Inserts a Transaction into the TransactionStore and
    /// emits an event to inform other modules that a Transaction
    /// has been added to the TransactionStore.
    // This is unneccessary under the system architecture, btw.
    pub(crate) async fn confirm_txn(&mut self, txn: Txn) -> Result<()> {
        let txn_hash = txn.id();

        info!("Storing transaction {txn_hash} in confirmed transaction store");

        //TODO: call checked methods instead
        self.database.insert_transaction(txn)?;

        let event = Event::TxnAddedToMempool(txn_hash);

        self.events_tx.send(event.into()).await?;

        Ok(())
    }

    pub fn _commit(&mut self) {
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
        let consolidated: HashSet<Txn> = {
            let nested: Vec<HashSet<Txn>> = proposals
                .iter()
                .map(|block| block.txns.iter().map(|(_, v)| v.clone().txn()).collect())
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
        let consolidated: HashSet<(U256, Claim)> = {
            let nested: Vec<HashSet<(U256, Claim)>> = {
                proposals
                    .iter()
                    .map(|block| block.claims.iter().map(|(k, v)| (*k, v.clone())).collect())
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

    pub fn _extend_accounts(&mut self, accounts: Vec<(Address, Account)>) -> Result<()> {
        self.database.extend_accounts(accounts);
        Ok(())
    }

    /// Returns a read handle for the StateStore to be able to read
    /// values from it.
    fn _get_state_store_handle(&self) -> StateStoreReadHandle {
        self.database.state_store_factory().handle()
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

    pub(crate) async fn handle_block_received(&mut self, block: Block) -> Result<()> {
        match block {
            Block::Genesis { block } => {
                if let Err(e) = self.dag.append_genesis(&block) {
                    let err_note = format!("Encountered GraphError: {e:?}");
                    return Err(NodeError::Other(err_note));
                };
            },
            Block::Proposal { block } => {
                if let Err(e) = self.dag.append_proposal(&block) {
                    let err_note = format!("Encountered GraphError: {e:?}");
                    return Err(NodeError::Other(err_note));
                }
            },
            Block::Convergence { block } => {
                if let Err(e) = self.dag.append_convergence(&block) {
                    let err_note = format!("Encountered GraphError: {e:?}");
                    return Err(NodeError::Other(err_note));
                }
                if block.certificate.is_none() {
                    if let Some(header) = self.dag.last_confirmed_block_header() {
                        if let Err(err) = self
                            .events_tx
                            .send(EventMessage::new(
                                None,
                                Event::ConvergenceBlockPrecheckRequested {
                                    convergence_block: block,
                                    block_header: header,
                                },
                            ))
                            .await
                        {
                            let err_note = format!(
                                "Failed to send EventMessage for PrecheckConvergenceBlock: {err}"
                            );
                            return Err(NodeError::Other(err_note));
                        }
                    }
                }
            },
        }
        Ok(())
    }

    pub(crate) fn block_certificate_created(&mut self, _certificate: Certificate) -> Result<()> {
        //
        //         let mut mine_block: Option<ConvergenceBlock> = None;
        //         let block_hash = certificate.block_hash.clone();
        //         if let Ok(Some(Block::Convergence { mut block })) =
        //             self.dag.write().map(|mut bull_dag| {
        //                 bull_dag
        //                     .get_vertex_mut(block_hash)
        //                     .map(|vertex| vertex.get_data())
        //             })
        //         {
        //             block.append_certificate(certificate.clone());
        //             self.last_confirmed_block_header = Some(block.get_header());
        //             mine_block = Some(block.clone());
        //         }
        //         if let Some(block) = mine_block {
        //             let proposal_block = Event::MineProposalBlock(
        //                 block.hash.clone(),
        //                 block.get_header().round,
        //                 block.get_header().epoch,
        //                 self.claim.clone(),
        //             );
        //             if let Err(err) = self
        //                 .events_tx
        //                 .send(EventMessage::new(None, proposal_block.clone()))
        //                 .await
        //             {
        //                 let err_msg = format!(
        //                     "Error occurred while broadcasting event {proposal_block:?}: {err:?}"
        //                 );
        //                 return Err(TheaterError::Other(err_msg));
        //             }
        //         } else {
        //             telemetry::debug!("Missing ConvergenceBlock for certificate: {certificate:?}");
        //         }
        //
        todo!()
    }

    pub fn handle_transaction_certificate_created(
        &mut self,
        txn: Box<Txn>,
    ) -> Result<()> {
        self.database
            .insert_transaction(*txn)
            .map_err(|err| NodeError::Other(err.to_string()))
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
    async fn mempool_snapshot(&self) -> Result<HashMap<TransactionDigest, Txn>> {
        self.mempool_snapshot().await
    }

    /// Get a transaction from state
    async fn get_transaction(&self, _transaction_digest: TransactionDigest) -> Result<Txn> {
        todo!()
    }

    /// List a group of transactions
    async fn list_transactions(
        &self,
        _digests: Vec<TransactionDigest>,
    ) -> Result<HashMap<TransactionDigest, Txn>> {
        todo!()
    }

    async fn get_account(&self, _address: Address) -> Result<Account> {
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

    async fn get_claims(&self, _claim_hashes: Vec<ClaimHash>) -> Result<Claims> {
        todo!()
    }

    async fn get_last_block(&self) -> Result<Block> {
        todo!()
    }

    fn state_store_values(&self) -> HashMap<Address, Account> {
        self.state_store_values()
    }

    fn transaction_store_values(&self) -> HashMap<TransactionDigest, Txn> {
        self.transaction_store_values()
    }

    fn claim_store_values(&self) -> HashMap<NodeId, Claim> {
        self.claim_store_values()
    }
}
