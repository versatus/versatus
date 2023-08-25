use std::collections::HashMap;

use integral_db::{JellyfishMerkleTreeWrapper, ReadHandleFactory};
use patriecia::{JellyfishMerkleTree, KeyHash, SimpleHasher, Version};
use storage_utils::{Result, StorageError};
use vrrb_core::txn::{TransactionDigest, Txn};

use crate::RocksDbAdapter;

#[derive(Debug, Clone)]
pub struct TransactionStoreReadHandle<H: SimpleHasher> {
    inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, H>,
}

impl<H: SimpleHasher> TransactionStoreReadHandle<H> {
    pub fn new(inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, H>) -> Self {
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

    pub fn entries(
        &self,
        version: Version,
        starting_key: KeyHash,
    ) -> Result<HashMap<TransactionDigest, Txn>> {
        // TODO: revisit and refactor into inner wrapper
        Ok(self
            .inner
            .iter(version, starting_key)
            .map_err(|e| StorageError::Other(e.to_string()))?
            .map(|Ok((key, value))| {
                let key = bincode::deserialize(&key.0).unwrap_or_default();
                let value = bincode::deserialize(&value).unwrap_or_default();

                (key, value)
            })
            .collect())
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
pub struct TransactionStoreReadHandleFactory<H: SimpleHasher> {
    inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, H>>,
}

impl<H: SimpleHasher> TransactionStoreReadHandleFactory<H> {
    pub fn new(inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, H>>) -> Self {
        Self { inner }
    }

    pub fn handle(&self) -> TransactionStoreReadHandle<H> {
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
