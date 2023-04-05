use std::{collections::HashMap, fmt::Display, path::PathBuf};

use lr_trie::H256;
use primitives::{Address, NodeId};
use serde_json::json;
use storage_utils::{Result, StorageError};
use vrrb_core::{
    account::{Account, UpdateArgs},
    claim::Claim,
    txn::Txn,
};

use crate::{
    ClaimStore,
    ClaimStoreReadHandleFactory,
    StateStore,
    StateStoreReadHandleFactory,
    TransactionStore,
    TransactionStoreReadHandleFactory,
    VrrbDbReadHandle,
};

#[derive(Debug, Clone)]
pub struct VrrbDbConfig {
    pub path: PathBuf,
    pub state_store_path: Option<String>,
    pub transaction_store_path: Option<String>,
    pub event_store_path: Option<String>,
    pub claim_store_path: Option<String>,
}

impl VrrbDbConfig {
    pub fn with_path(&mut self, path: PathBuf) -> Self {
        self.path = path;

        self.clone()
    }
}

impl VrrbDbConfig {
    pub fn with_path(&mut self, path: PathBuf) -> Self {
        self.path = path;

        self.clone()
    }
}

impl Default for VrrbDbConfig {
    fn default() -> Self {
        let path = storage_utils::get_node_data_dir()
            .unwrap_or_default()
            .join("db");

        Self {
            path,
            state_store_path: None,
            transaction_store_path: None,
            event_store_path: None,
            claim_store_path: None,
        }
    }
}

#[derive(Debug)]
pub struct VrrbDb {
    state_store: StateStore,
    transaction_store: TransactionStore,
    claim_store: ClaimStore,
}

impl VrrbDb {
    pub fn new(config: VrrbDbConfig) -> Self {
        let state_store = StateStore::new(&config.path);
        let transaction_store = TransactionStore::new(&config.path);
        let claim_store = ClaimStore::new(&config.path);

        Self {
            state_store,
            transaction_store,
            claim_store,
        }
    }

    pub fn read_handle(&self) -> VrrbDbReadHandle {
        VrrbDbReadHandle::new(
            self.state_store.factory(),
            self.transaction_store_factory(),
            self.claim_store_factory(),
        )
    }

    pub fn new_with_stores(
        state_store: StateStore,
        transaction_store: TransactionStore,
        claim_store: ClaimStore,
    ) -> Self {
        Self {
            state_store,
            transaction_store,
            claim_store,
        }
    }

    pub fn state_store(&self) -> &StateStore {
        &self.state_store
    }

    pub fn transaction_store(&self) -> &TransactionStore {
        &self.transaction_store
    }

    pub fn claim_store(&self) -> &ClaimStore {
        &self.claim_store
    }

    /// Returns the current state store trie's root hash.
    pub fn state_root_hash(&self) -> Option<H256> {
        self.state_store.root_hash()
    }

    /// Returns the transaction store trie's root hash.
    pub fn transactions_root_hash(&self) -> Option<H256> {
        self.transaction_store.root_hash()
    }

    /// Returns the claim store trie's root hash.
    pub fn claims_root_hash(&self) -> Option<H256> {
        self.claim_store.root_hash()
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the state trie.
    pub fn state_store_factory(&self) -> StateStoreReadHandleFactory {
        self.state_store.factory()
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the the transaction trie.
    pub fn transaction_store_factory(&self) -> TransactionStoreReadHandleFactory {
        self.transaction_store.factory()
    }

    /// Produces a reader factory that can be used to generate read_handles into
    /// the claim trie
    pub fn claim_store_factory(&self) -> ClaimStoreReadHandleFactory {
        self.claim_store.factory()
    }

    /// Inserts an account to current state tree.
    pub fn insert_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.state_store.insert(key, account)
    }

    /// Adds multiplpe accounts to current state tree.
    pub fn extend_accounts(&mut self, accounts: Vec<(Address, Account)>) {
        self.state_store.extend(accounts);
    }

    /// Updates an account on the current state tree.
    pub fn update_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.state_store
            .update(
                key,
                UpdateArgs {
                    nonce: account.nonce + 1,
                    credits: Some(account.credits),
                    debits: Some(account.debits),
                    storage: Some(account.storage),
                    code: Some(account.code),
                    digests: Some(account.digests),
                },
            )
            .map_err(|err| StorageError::Other(err.to_string()))
    }

    /// Inserts a confirmed transaction to the ledger. Does not check if
    /// accounts involved in the transaction actually exist.
    pub fn insert_transaction_unchecked(&mut self, txn: Txn) -> Result<()> {
        self.transaction_store.insert(txn)
    }

    /// Adds multiplpe transactions to current state tree. Does not check if
    /// accounts involved in the transaction actually exist.
    pub fn extend_transactions_unchecked(&mut self, transactions: Vec<Txn>) {
        self.transaction_store.extend(transactions);
    }

    /// Inserts a confirmed transaction to the ledger. Does not check if
    /// accounts involved in the transaction actually exist.
    pub fn insert_transaction(&mut self, txn: Txn) -> Result<()> {
        self.transaction_store.insert(txn)
    }

    /// Adds multiplpe transactions to current transaction tree. Does not check
    /// if accounts involved in the transaction actually exist.
    pub fn extend_transactions(&mut self, transactions: Vec<Txn>) {
        self.transaction_store.extend(transactions);
    }

    /// Inserts a confirmed claim to the current claim tree.
    pub fn insert_claim_unchecked(&mut self, node_id: NodeId, claim: Claim) -> Result<()> {
        self.claim_store.insert(node_id, claim)
    }

    /// Adds multiple claims to the current claim tree.  
    pub fn extend_claims_unchecked(&mut self, claims: Vec<(NodeId, Claim)>) {
        self.claim_store.extend(claims)
    }

    /// Inserts a confirmed claim into the claim tree.
    pub fn insert_claim(&mut self, node_id: NodeId, claim: Claim) -> Result<()> {
        self.claim_store.insert(node_id, claim)
    }

    /// Inserts multiple claims into the current claim trie
    pub fn extend_claims(&mut self, claims: Vec<(NodeId, Claim)>) {
        self.claim_store.extend(claims)
    }
}

impl Clone for VrrbDb {
    fn clone(&self) -> VrrbDb {
        Self {
            state_store: self.state_store.clone(),
            transaction_store: self.transaction_store.clone(),
            claim_store: self.claim_store.clone(),
        }
    }
}

impl Display for VrrbDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_entries = self.state_store_factory().handle().entries();
        let transaction_entries = self
            .transaction_store_factory()
            .handle()
            .entries()
            .into_iter()
            .map(|(digest, txn)| (digest.to_string(), txn))
            .collect::<HashMap<String, Txn>>();
        let claim_entries = self.claim_store_factory().handle().entries();

        let out = json!({
            "state": {
                "count": state_entries.len(),
                "entries": state_entries,
            },
            "transactions": {
                "count": transaction_entries.len(),
                "entries": transaction_entries,
            },
            "claims": {
                "count": claim_entries.len(),
                "entries": claim_entries,
            },
        });

        //TODO: report errors better
        let out_str = serde_json::to_string_pretty(&out).map_err(|_| std::fmt::Error)?;

        f.write_str(&out_str)
    }
}
