use std::{collections::HashMap, path::PathBuf};

use lr_trie::H256;
use primitives::Address;
use storage_utils::{Result, StorageError};
use vrrb_core::account::{Account, UpdateArgs};

use crate::{
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
}

impl Default for VrrbDbConfig {
    fn default() -> Self {
        let path = storage_utils::get_node_data_dir()
            .unwrap_or_default()
            .join("node")
            .join("db");

        Self {
            path,
            state_store_path: None,
            transaction_store_path: None,
            event_store_path: None,
        }
    }
}

#[derive(Debug)]
pub struct VrrbDb {
    state_store: StateStore,
    transaction_store: TransactionStore,
}

impl VrrbDb {
    pub fn new(config: VrrbDbConfig) -> Self {
        let state_store = StateStore::new(&config.path);
        let transaction_store = TransactionStore::new(&config.path);

        Self {
            state_store,
            transaction_store,
        }
    }

    pub fn read_handle(&self) -> VrrbDbReadHandle {
        VrrbDbReadHandle::new(self.state_store.factory(), self.transaction_store_factory())
    }

    pub fn new_with_stores(state_store: StateStore, transaction_store: TransactionStore) -> Self {
        Self {
            state_store,
            transaction_store,
        }
    }

    pub fn state_store(&self) -> &StateStore {
        &self.state_store
    }

    pub fn transaction_store(&self) -> &TransactionStore {
        &self.transaction_store
    }

    /// Returns the current state store trie's root hash.
    pub fn state_root_hash(&self) -> Option<H256> {
        self.state_store.root_hash()
    }

    /// Returns the transaction store trie's root hash.
    pub fn transactions_root_hash(&self) -> Option<H256> {
        self.transaction_store.root_hash()
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

    /// Returns a mappig of public keys and accounts.
    pub fn entries(&self) -> HashMap<Address, Account> {
        self.state_store.read_handle().entries()
    }

    /// Inserts an account to current state tree.
    pub fn insert_account(&mut self, key: Address, account: Account) -> Result<()> {
        self.state_store
            .insert(key, account)
            .map_err(|err| StorageError::Other(err.to_string()))
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
                },
            )
            .map_err(|err| StorageError::Other(err.to_string()))
    }
}

impl Clone for VrrbDb {
    fn clone(&self) -> VrrbDb {
        Self {
            state_store: self.state_store.clone(),
            transaction_store: self.transaction_store.clone(),
        }
    }
}
