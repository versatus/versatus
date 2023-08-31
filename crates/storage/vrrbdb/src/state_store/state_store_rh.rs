use std::collections::HashMap;

use integral_db::{JellyfishMerkleTreeWrapper, ReadHandleFactory};
use patriecia::{JellyfishMerkleTree, KeyHash};
use primitives::Address;
use sha2::Sha256;
use storage_utils::{Result, StorageError};
use vrrb_core::account::Account;

use crate::{RocksDbAdapter, STARTING_KEY};

#[derive(Debug, Clone)]
pub struct StateStoreReadHandle {
    pub inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, Sha256>,
}

impl StateStoreReadHandle {
    pub fn new(inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, Sha256>) -> Self {
        Self { inner }
    }

    /// Returns `Some(Account)` if an account exist under given PublicKey.
    /// Otherwise returns `None`.
    pub fn get(&self, key: &Address) -> Result<Account> {
        self.inner
            .get(key, self.inner.version())
            .map_err(|err| StorageError::Other(err.to_string()))
    }

    /// Get a batch of accounts by providing Vec of PublicKeysHash
    ///
    /// Returns HashMap indexed by PublicKeys and containing either
    /// Some(account) or None if account was not found.
    pub fn batch_get(
        &self,
        keys: Vec<Address>,
    ) -> HashMap<Address, Option<Account>> {
        let mut accounts = HashMap::new();

        keys.iter().for_each(|key| {
            let value = self.get(key).ok();
            accounts.insert(key.to_owned(), value);
        });

        accounts
    }

    pub fn entries(&self, starting_key_opt: Option<KeyHash>) -> HashMap<Address, Account> { 
        // TODO: revisit and refactor into inner wrapper
        let starting_key = if let Some(key_hash) = starting_key_opt {
            key_hash
        } else {
            STARTING_KEY
        };
        self.inner
            .iter(self.inner.version(), starting_key).expect("unable to create iterator from merkle tree wrapper starting at key {starting_key} with version {version}")
            .filter_map(|item| {
                if let Ok((key, value)) = item {
                    if let Ok(key) = bincode::deserialize(&key.0) {
                        let value = bincode::deserialize(&value).unwrap_or_default();
    
                        return Some((key, value));
                    }
                }
                None
            })
            .collect()
    }

    /// Returns a number of initialized accounts in the database
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the information about the StateDb being empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct StateStoreReadHandleFactory {
    inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, Sha256>>,
}

impl StateStoreReadHandleFactory {
    pub fn new(inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, Sha256>>) -> Self {
        Self { inner }
    }

    pub fn handle(&self) -> StateStoreReadHandle {
        let handle = self
            .inner
            .handle()
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let inner = JellyfishMerkleTreeWrapper::new(handle);

        StateStoreReadHandle { inner }
    }
}
