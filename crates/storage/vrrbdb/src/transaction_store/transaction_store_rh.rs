use std::collections::HashMap;

use integral_db::{JellyfishMerkleTreeWrapper, ReadHandleFactory};
use patriecia::{JellyfishMerkleTree, KeyHash, Version};
use sha2::Sha256;
use storage_utils::{Result, StorageError};
use vrrb_core::txn::{TransactionDigest, Txn};

use crate::RocksDbAdapter;

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

    pub fn entries(&self, starting_key_opt: Option<KeyHash>) -> HashMap<TransactionDigest, Txn> {
        // TODO: revisit and refactor into inner wrapper
        let starting_key = if let Some(key_hash) = starting_key_opt {
            key_hash
        } else {
            KeyHash::sha256::<TransactionDigest>()
        };
        self.inner
            .iter(self.inner.version(), starting_key)
            .unwrap()
            .filter_map(|item| {
                if let Ok((_, txn)) = item {
                    let txn = bincode::deserialize::<Txn>(&txn).unwrap_or_default();

                    return Some((txn.digest().clone(), txn));
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
