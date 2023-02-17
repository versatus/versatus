use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};

use lr_trie::{InnerTrieWrapper, LeftRightTrie, H256};
use patriecia::db::MemoryDB;
use primitives::{TransactionDigest, TxHash};
use storage_utils::{Result, StorageError};
use vrrb_core::txn::Txn;

use crate::RocksDbAdapter;

mod transaction_store_rh;
pub use transaction_store_rh::*;

#[derive(Debug, Clone)]
pub struct TransactionStore {
    trie: LeftRightTrie<'static, TransactionDigest, Txn, RocksDbAdapter>,
}

impl Default for TransactionStore {
    fn default() -> Self {
        let db_path = storage_utils::get_node_data_dir()
            .unwrap_or_default()
            .join("node")
            .join("db")
            .join("transactions");

        let db_adapter = RocksDbAdapter::new(db_path, "transactions").unwrap_or_default();

        let trie = LeftRightTrie::new(Arc::new(db_adapter));

        Self { trie }
    }
}

impl TransactionStore {
    /// Returns new, empty instance of TransactionStore
    pub fn new(path: &PathBuf) -> Self {
        let db_adapter = RocksDbAdapter::new(path.to_owned(), "transactions").unwrap_or_default();
        let trie = LeftRightTrie::new(Arc::new(db_adapter));

        Self { trie }
    }

    pub fn factory(&self) -> TransactionStoreReadHandleFactory {
        let inner = self.trie.factory();

        TransactionStoreReadHandleFactory::new(inner)
    }

    pub fn read_handle(&self) -> TransactionStoreReadHandle {
        let inner = self.trie.handle();
        TransactionStoreReadHandle::new(inner)
    }

    pub fn insert(&mut self, txn: Txn) -> Result<()> {
        let key = TransactionDigest::from(txn.clone());

        Ok(self.trie.insert(key, txn))
    }

    pub fn root_hash(&self) -> Option<H256> {
        self.trie.root()
    }
}
