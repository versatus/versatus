use std::collections::HashMap;

use block::{Block, ClaimHash};
use primitives::{Address, NodeId, Round};
use storage::{
    storage_utils::StorageError,
    vrrbdb::{Claims, VrrbDb, VrrbDbReadHandle},
};
use vrrb_core::{
    account::Account,
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

use crate::{state_reader::StateReader, Result};

#[async_trait::async_trait]
pub trait StateStore<S: StateReader> {
    type Error;

    fn state_reader(&self) -> S;
}

#[async_trait::async_trait]
impl StateStore<VrrbDbReadHandle> for VrrbDb {
    type Error = StorageError;

    fn state_reader(&self) -> VrrbDbReadHandle {
        self.read_handle()
    }
}

#[async_trait::async_trait]
impl StateReader for VrrbDbReadHandle {
    /// Returns a full list of all accounts within state
    async fn state_snapshot(&self) -> Result<HashMap<Address, Account>> {
        todo!()
    }

    /// Returns a full list of transactions pending to be confirmed
    async fn mempool_snapshot(&self) -> Result<HashMap<TransactionDigest, Txn>> {
        todo!()
    }

    /// Get a transaction from state
    async fn get_transaction(&self, transaction_digest: TransactionDigest) -> Result<Txn> {
        todo!()
    }

    /// List a group of transactions
    async fn list_transactions(
        &self,
        digests: Vec<TransactionDigest>,
    ) -> Result<HashMap<TransactionDigest, Txn>> {
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

    fn transaction_store_values(&self) -> HashMap<TransactionDigest, Txn> {
        self.transaction_store_values()
    }

    fn claim_store_values(&self) -> HashMap<NodeId, Claim> {
        self.claim_store_values()
    }
}
