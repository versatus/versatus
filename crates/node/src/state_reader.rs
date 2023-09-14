use std::collections::HashMap;

use block::{Block, ClaimHash};
use primitives::{Address, NodeId, Round};
use storage::vrrbdb::Claims;
use vrrb_core::{
    account::Account,
    claim::Claim,
};
use vrrb_core::transactions::{TransactionDigest, TransactionKind};

use crate::Result;

#[async_trait::async_trait]
pub trait StateReader: std::fmt::Debug {
    /// Returns a full list of all accounts within state
    async fn state_snapshot(&self) -> Result<HashMap<Address, Account>>;

    /// Returns a full list of transactions pending to be confirmed
    async fn mempool_snapshot(&self) -> Result<HashMap<TransactionDigest, TransactionKind>>;

    /// Get a transaction from state
    async fn get_transaction(&self, transaction_digest: TransactionDigest) -> Result<TransactionKind>;

    /// List a group of transactions
    async fn list_transactions(
        &self,
        digests: Vec<TransactionDigest>,
    ) -> Result<HashMap<TransactionDigest, TransactionKind>>;

    async fn get_account(&self, address: Address) -> Result<Account>;

    async fn get_round(&self) -> Result<Round>;

    async fn get_blocks(&self) -> Result<Vec<Block>>;

    async fn get_transaction_count(&self) -> Result<usize>;

    async fn get_claims_by_account_id(&self) -> Result<Vec<Claim>>;

    async fn get_claim_hashes(&self) -> Result<Vec<ClaimHash>>;

    async fn get_claims(&self, claim_hashes: Vec<ClaimHash>) -> Result<Claims>;

    async fn get_last_block(&self) -> Result<Block>;

    /// Returns a copy of all values stored within the state trie
    fn state_store_values(&self) -> HashMap<Address, Account>;

    /// Returns a copy of all values stored within the state trie
    fn transaction_store_values(&self) -> HashMap<TransactionDigest, TransactionKind>;

    fn claim_store_values(&self) -> HashMap<NodeId, Claim>;
}
