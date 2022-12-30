/// This module contains the Network State struct (which will be replaced with
/// the Left-Right State Trie)
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use derive_builder::Builder;
use lr_trie::{Key, LeftRightTrie, ReadHandleFactory, H256};
use lrdb::Account;
use patriecia::{db::MemoryDB, inner::InnerTrie, trie::Trie};
use primitives::PublicKey;
use serde::{Deserialize, Serialize};
use vrrb_core::txn::Txn;

use crate::{result::Result, types::StatePath, StateError};

const DEFAULT_SERIALIZED_STATE_FILENAME: &str = "state";
const DEFAULT_SERIALIZED_CONFIRMED_TXNS_FILENAME: &str = "txns";
const DEFAULT_SERIALIZED_MEMPOOL_FILENAME: &str = "mempool";

#[derive(Debug, Clone, Default)]
pub struct NodeStateConfig {
    pub path: StatePath,
    pub serialized_state_filename: Option<String>,
    pub serialized_mempool_filename: Option<String>,
    pub serialized_confirmed_txns_filename: Option<String>,
}

/// The Node State struct, contains basic information required to determine
/// the current state of the network.
#[derive(Debug)]
pub struct NodeState {
    /// Path to database
    pub path: StatePath,

    /// VRRB world state. it contains the accounts tree
    state_trie: LeftRightTrie<MemoryDB>,

    /// Confirmed transactions
    tx_trie: LeftRightTrie<MemoryDB>,

    /// Unconfirmed transactions
    mempool: LeftRightTrie<MemoryDB>,
}

impl Clone for NodeState {
    fn clone(&self) -> NodeState {
        NodeState {
            path: self.path.clone(),
            state_trie: self.state_trie.clone(),
            tx_trie: self.tx_trie.clone(),
            mempool: self.tx_trie.clone(),
        }
    }
}

impl From<NodeStateValues> for NodeState {
    fn from(node_state_values: NodeStateValues) -> Self {
        let mut state_trie = LeftRightTrie::new(Arc::new(MemoryDB::new(true)));
        let mut tx_trie = LeftRightTrie::new(Arc::new(MemoryDB::new(true)));

        let mapped_state = node_state_values
            .state
            .into_iter()
            .map(|(key, acc)| (key.into_bytes(), acc))
            .collect();

        state_trie.extend(mapped_state);

        let mapped_txns = node_state_values
            .txns
            .into_iter()
            .map(|(key, acc)| (key, acc))
            .collect();

        tx_trie.extend(mapped_txns);

        Self {
            path: PathBuf::new(),
            state_trie,
            tx_trie,
            mempool: LeftRightTrie::new(Arc::new(MemoryDB::new(true))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeStateValues {
    pub txns: HashMap<PublicKey, Account>,
    pub state: HashMap<String, Account>,
}

impl From<&NodeState> for NodeStateValues {
    fn from(node_state: &NodeState) -> Self {
        let state = node_state
            .entries()
            .into_iter()
            .map(|(k, v)| (format!("{:?}", k), v))
            .collect();

        Self {
            txns: HashMap::new(),
            state,
        }
    }
}

impl NodeStateValues {
    /// Converts a vector of bytes into a Network State or returns an error if
    /// it's unable to
    #[allow(dead_code)]
    pub fn from_bytes(data: &[u8]) -> Result<NodeStateValues> {
        serde_json::from_slice::<NodeStateValues>(data)
            .map_err(|err| StateError::Other(err.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct NodeStateReadHandle {
    state_handle_factory: ReadHandleFactory<InnerTrie<MemoryDB>>,
    tx_handle_factory: ReadHandleFactory<InnerTrie<MemoryDB>>,
    mempool_handle_factory: ReadHandleFactory<InnerTrie<MemoryDB>>,
}

impl NodeStateReadHandle {
    /// Returns a copy of all values stored within the state trie
    pub fn values(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.state_handle_factory
            .handle()
            .enter()
            .map(|guard| guard.values())
            .unwrap_or_else(|| Ok(vec![]))
            .map_err(|err| StateError::Other(err.to_string()))
    }

    /// Returns a copy of all values stored within the mempool trie
    pub fn mempool_values(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.mempool_handle_factory
            .handle()
            .enter()
            .map(|guard| guard.values())
            .unwrap_or_else(|| Ok(vec![]))
            .map_err(|err| StateError::Other(err.to_string()))
    }

    /// Returns a copy of all values stored within the confirmed transactions trie
    pub fn confirmed_txn_values(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.tx_handle_factory
            .handle()
            .enter()
            .map(|guard| guard.values())
            .unwrap_or_else(|| Ok(vec![]))
            .map_err(|err| StateError::Other(err.to_string()))
    }
}

impl NodeState {
    pub fn new(cfg: &NodeStateConfig) -> Self {
        let path = cfg.path.clone();

        if let Some(serialized_state_filename) = &cfg.serialized_state_filename {
            telemetry::info!("restoring state from file {serialized_state_filename}");
        }

        // TODO: replace memorydb with real backing db later
        let mem_db = MemoryDB::new(true);

        let state_backing_db = Arc::new(mem_db.clone());
        let state_trie = LeftRightTrie::new(state_backing_db);

        let tx_backing_db = Arc::new(mem_db.clone());
        let tx_trie = LeftRightTrie::new(tx_backing_db);

        let mempool_backing_db = Arc::new(mem_db);
        let mempool = LeftRightTrie::new(mempool_backing_db);

        Self {
            path,
            state_trie,
            tx_trie,
            mempool,
        }
    }

    /// Dumps a hex string representation of `NodeStateValues` to file.
    pub fn dump_to_file(&self) -> Result<()> {
        //TODO: discuss if hex::encode is worth implementing
        unimplemented!()
    }

    /// Generates a backup of NodeState serialized into JSON at the specified
    /// path.
    pub fn serialize_to_json(&self) -> Result<()> {
        let node_state_values = NodeStateValues::from(self);
        let serialized = serde_json::to_vec(&node_state_values)
            .map_err(|err| StateError::Other(err.to_string()))?;

        fs::write(&self.path, serialized).map_err(|err| StateError::Other(err.to_string()))?;

        Ok(())
    }

    /// Restores the network state from a serialized file stored on disk.
    pub fn restore(path: &PathBuf) -> Result<NodeState> {
        //NOTE: refactor this naive impl later
        let ext = path
            .extension()
            .ok_or_else(|| {
                StateError::Other(format!("file extension not found on file {:?}", path))
            })?
            .to_str()
            .ok_or_else(|| {
                StateError::Other("file extension is not a valid UTF-8 string".to_string())
            })?;

        match ext {
            // TODO: add more match arms to support more backup filetypes
            "json" => NodeState::restore_from_json_file(path),
            _ => Err(StateError::Other(format!(
                "file extension not found on file {:?}",
                &path
            ))),
        }
    }

    fn restore_from_json_file(path: &PathBuf) -> Result<NodeState> {
        let read = fs::read(path).map_err(|err| StateError::Other(err.to_string()))?;

        let deserialized: NodeStateValues =
            serde_json::from_slice(&read).map_err(|err| StateError::Other(err.to_string()))?;

        let mut node_state = NodeState::from(deserialized);
        node_state.path = path.to_owned();

        Ok(node_state)
    }

    /// Returns the current state trie's root hash.
    pub fn root_hash(&self) -> Option<H256> {
        self.state_trie.root()
    }

    pub fn read_handle(&self) -> NodeStateReadHandle {
        let state_handle_factory = self.state_trie.factory();
        let tx_handle_factory = self.tx_trie.factory();
        let mempool_handle_factory = self.mempool.factory();

        NodeStateReadHandle {
            state_handle_factory,
            tx_handle_factory,
            mempool_handle_factory,
        }
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the state tree.
    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<MemoryDB>> {
        self.state_trie.factory()
    }

    /// Returns a mappig of public keys and accounts.
    pub fn entries(&self) -> HashMap<PublicKey, Account> {
        self.state_trie
            .handle()
            .iter()
            .map(|(k, v)| {
                let account: Account = serde_json::from_slice(&v).unwrap_or_default();
                (k, account)
            })
            .collect::<HashMap<Key, Account>>()
    }

    /// Retrieves an account entry from the current state tree.
    pub fn get_account(&mut self, key: &PublicKey) -> Result<Account> {
        let raw_account_bytes = self
            .state_trie
            .handle()
            .get(key)
            .unwrap_or_default()
            .unwrap_or_default(); //TODO: Refactor patriecia to only return results, not options

        let account = serde_json::from_slice(&raw_account_bytes).unwrap_or_default();

        Ok(account)
    }

    /// Adds an account to current state tree.
    pub fn add_account(&mut self, key: PublicKey, account: Account) {
        self.state_trie.add(key, account);
    }

    /// Inserts an account to current state tree.
    pub fn insert_account(&mut self, key: PublicKey, account: Account) {
        self.state_trie.add(key, account);
    }

    /// Adds multiplpe accounts to current state tree.
    pub fn extend_accounts(&mut self, accounts: Vec<(PublicKey, Account)>) {
        self.state_trie.extend(accounts);
    }

    /// Updates an account on the current state tree.
    pub fn update_account(&mut self, _key: PublicKey, _account: Account) {
        todo!()
    }

    /// Removes an account from the current state tree.
    pub fn remove_account(&mut self, _key: PublicKey) {
        todo!()
    }

    /// Adds a transaction to mempool.
    pub fn add_txn_to_mempool(&mut self, txn: Txn) {
        self.mempool.add(txn.digest_bytes(), txn);
    }

    /// Removes a transaction to mempool.
    pub fn remove_txn_from_mempool(&mut self, txn: Txn) {
        self.mempool.add(txn.digest_bytes(), txn);
    }

    /// Adds a transaction to the confirmed transactions store.
    pub fn add_txn_to_confirmed_trie(&mut self, txn: Txn) {
        self.tx_trie.add(txn.digest_bytes(), txn);
    }
}
