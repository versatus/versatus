use std::collections::HashMap;
use std::ffi::OsStr;
use std::os;
use std::path::PathBuf;
use std::{fs, sync::Arc};

use accountable::accountable::Accountable;
/// This module contains the Network State struct (which will be replaced with
/// the Left-Right State Trie)
use lr_trie::{Key, LeftRightTrie, ReadHandleFactory, H256};
use lrdb::Account;
use patriecia::db::MemoryDB;
use patriecia::inner::InnerTrie;
use patriecia::trie::Trie;
use primitives::PublicKey;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use telemetry::{error, info};

use crate::result::Result;
use crate::types::{
    CreditsHash, CreditsRoot, DebitsHash, DebitsRoot, LedgerBytes, StateHash, StatePath,
    StateRewardState, StateRoot,
};
use crate::StateError;

/// The Node State struct, contains basic information required to determine
/// the current state of the network.
#[derive(Debug)]
pub struct NodeState {
    /// Path to database
    pub path: StatePath,
    state_trie: LeftRightTrie<MemoryDB>,
    tx_trie: LeftRightTrie<MemoryDB>,
}

impl Clone for NodeState {
    /// Warning: do not use yet as lr_trie doesn't fully implement clone yet.
    fn clone(&self) -> NodeState {
        NodeState {
            path: self.path.clone(),
            state_trie: self.state_trie.clone(),
            tx_trie: self.tx_trie.clone(),
        }
    }
}

impl From<NodeStateValues> for NodeState {
    fn from(node_state_values: NodeStateValues) -> Self {
        let mut state_trie = LeftRightTrie::new(Arc::new(MemoryDB::new(true)));

        let mapped_state = node_state_values
            .state
            .into_iter()
            .map(|(key, acc)| (key.into_bytes(), acc))
            .collect();

        state_trie.extend(mapped_state);

        Self {
            path: PathBuf::new(),
            state_trie,
            tx_trie: LeftRightTrie::new(Arc::new(MemoryDB::new(true))),
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
    pub fn from_bytes(data: &[u8]) -> Result<NodeStateValues> {
        serde_json::from_slice::<NodeStateValues>(data)
            .map_err(|err| StateError::Other(err.to_string()))
    }
}

impl NodeState {
    pub fn new(path: std::path::PathBuf) -> Self {
        // TODO: replace memorydb with real backing db later
        let mem_db = MemoryDB::new(true);
        let backing_db = Arc::new(mem_db);
        let state_trie = LeftRightTrie::new(backing_db.clone());
        let tx_trie = LeftRightTrie::new(backing_db);

        Self {
            path,
            state_trie,
            tx_trie,
        }
    }

    /// Dumps a hex string representation of `NodeStateValues` to file.
    pub fn dump_to_file(&self) -> Result<()> {
        //TODO: discuss if hex::encode is worth implementing
        unimplemented!()
    }

    /// Generates a backup of NodeState serialized into JSON at the specified path.
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

    /// Produces a reader factory that can be used to generate read handles into the state tree.
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

    /// Adds multiplpe accounts to current state tree.
    pub fn extend_accounts(&mut self, accounts: Vec<(PublicKey, Account)>) {
        self.state_trie.extend(accounts);
    }

    /// Updates an account on the current state tree.
    pub fn update_account(&mut self, key: PublicKey, account: Account) {
        todo!()
    }

    /// Removes an account from the current state tree.
    pub fn remove_account(&mut self, key: PublicKey) {
        todo!()
    }
}
