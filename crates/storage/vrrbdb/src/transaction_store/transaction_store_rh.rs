use std::collections::HashMap;

use integral_db::{JellyfishMerkleTreeWrapper, ReadHandleFactory};
use patriecia::{JellyfishMerkleTree, Version};
use sha2::Sha256;
use storage_utils::{Result, StorageError};
use vrrb_core::txn::{TransactionDigest, Txn};

use crate::{RocksDbAdapter, STARTING_KEY};

#[derive(Debug, Clone)]
pub struct TransactionStoreReadHandle {
    inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, Sha256>,
}

impl TransactionStoreReadHandle {
    pub fn new(inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, Sha256>) -> Self {
        Self { inner }
    }

    pub fn get(&self, key: &TransactionDigest, version: Version) -> Result<Txn> {
        self.inner
            .get(key, version)
            .map_err(|err| StorageError::Other(err.to_string()))
    }

    pub fn batch_get(
        &self,
        keys: Vec<TransactionDigest>,
        version: Version,
    ) -> HashMap<TransactionDigest, Option<Txn>> {
        let mut transactions = HashMap::new();

        keys.iter().for_each(|key| {
            let value = self.get(key, version).ok();
            transactions.insert(key.to_owned(), value);
        });

        transactions
    }

    pub fn entries(&self) -> HashMap<TransactionDigest, Txn> {
        // TODO: revisit and refactor into inner wrapper
        self.inner
            .iter(self.inner.version(), STARTING_KEY)
            .unwrap()
            .filter_map(|item| {
                if let Ok((key, value)) = item {
                    let key = bincode::deserialize(&key.0).unwrap_or_default();
                    let value = bincode::deserialize(&value).unwrap_or_default();

                    return Some((key, value));
                }
                None
            })
            .collect()
    }

    /// Returns a number of transactions in the ledger
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if TransactionStore is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct TransactionStoreReadHandleFactory {
    inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, Sha256>>,
}

impl TransactionStoreReadHandleFactory {
    pub fn new(inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, Sha256>>) -> Self {
        Self { inner }
    }

    pub fn handle(&self) -> TransactionStoreReadHandle {
        let handle = self
            .inner
            .handle()
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let inner = JellyfishMerkleTreeWrapper::new(handle);

        TransactionStoreReadHandle { inner }
    }
}
