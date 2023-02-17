use std::{collections::HashMap, sync::Arc, time::SystemTime};

use lr_trie::{InnerTrieWrapper, LeftRightTrie, ReadHandleFactory};
use patriecia::{db::MemoryDB, inner::InnerTrie};
use primitives::{TransactionDigest, TxHash};
use storage_utils::{Result, StorageError};
use vrrb_core::txn::Txn;

use crate::RocksDbAdapter;

#[derive(Debug, Clone)]
pub struct TransactionStoreReadHandle {
    inner: InnerTrieWrapper<RocksDbAdapter>,
}

impl TransactionStoreReadHandle {
    pub fn new(inner: InnerTrieWrapper<RocksDbAdapter>) -> Self {
        Self { inner }
    }

    pub fn get(&self, key: &TransactionDigest) -> Result<Txn> {
        self.inner
            .get(key)
            .map_err(|err| StorageError::Other(err.to_string()))
    }

    pub fn batch_get(
        &self,
        keys: Vec<TransactionDigest>,
    ) -> HashMap<TransactionDigest, Option<Txn>> {
        let mut transactions = HashMap::new();

        keys.iter().for_each(|key| {
            let value = self.get(key).ok();
            transactions.insert(key.to_owned(), value);
        });

        transactions
    }

    pub fn entries(&self) -> HashMap<TransactionDigest, Txn> {
        // TODO: revisit and refactor into inner wrapper
        self.inner
            .iter()
            .map(|(key, value)| {
                let key = bincode::deserialize(&key).unwrap_or_default();
                let value = bincode::deserialize(&value).unwrap_or_default();

                (key, value)
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
    inner: ReadHandleFactory<InnerTrie<RocksDbAdapter>>,
}

impl TransactionStoreReadHandleFactory {
    pub fn new(inner: ReadHandleFactory<InnerTrie<RocksDbAdapter>>) -> Self {
        Self { inner }
    }

    pub fn handle(&self) -> TransactionStoreReadHandle {
        let handle = self
            .inner
            .handle()
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let inner = InnerTrieWrapper::new(handle);

        TransactionStoreReadHandle { inner }
    }
}
