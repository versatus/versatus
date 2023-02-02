/// This module contains the Network State struct (which will be replaced with
/// the Left-Right State Trie)
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use lr_trie::{Key, LeftRightTrie, ReadHandleFactory, H256};
use lrdb::{StateDb, StateDbReadHandleFactory, TxnDb};
use mempool::{LeftRightMempool, Mempool, MempoolReadHandleFactory, PoolType};
use patriecia::{db::MemoryDB, inner::InnerTrie, trie::Trie};
use primitives::{
    node,
    ByteSlice,
    ByteVec,
    PublicKey,
    SerializedPublicKey,
    SerializedPublicKeyString,
    TxHash,
    TxHashString,
};
use serde::{Deserialize, Serialize};
use telemetry::info;
use vrrb_core::{
    account::{Account, UpdateArgs},
    serde_helpers,
    txn::Txn,
};

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

    // TODO: change lifetime parameter once refactoring is complete
    state_db: StateDb<'static>,
    txn_db: TxnDb<'static>,
    mempool: LeftRightMempool,
}

impl NodeState {
    pub fn new(cfg: &NodeStateConfig) -> Self {
        let path = cfg.path.clone();

        if let Some(serialized_state_filename) = &cfg.serialized_state_filename {
            info!("restoring state from file {serialized_state_filename}");
        }

        let mut state_db = StateDb::new();
        let mut txn_db = TxnDb::new();

        // TODO: replace memorydb with real backing db later
        let mem_db = MemoryDB::new(true);
        let backing_db = Arc::new(mem_db);

        let mempool = LeftRightMempool::new();

        Self {
            path,
            state_db,
            txn_db,
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
    pub fn state_root_hash(&self) -> Option<H256> {
        self.state_db.root_hash()
    }

    pub fn read_handle(&self) -> NodeStateReadHandle {
        let state_handle_factory = self.state_db_factory();
        let mempool_handle_factory = self.mempool_handle_factory();

        NodeStateReadHandle {
            state_handle_factory,
            mempool_handle_factory,
        }
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the state tree.
    pub fn state_db_factory(&self) -> StateDbReadHandleFactory {
        self.state_db.factory()
    }

    /// Produces a reader factory that can be used to generate read handles into
    /// the node's mempool.
    pub fn mempool_handle_factory(&self) -> MempoolReadHandleFactory {
        self.mempool.factory()
    }

    /// Returns a mappig of public keys and accounts.
    pub fn entries(&self) -> HashMap<SerializedPublicKeyString, Account> {
        self.state_db.read_handle().entries()
    }

    /// Retrieves an account entry from the current state tree.
    pub fn get_account(&mut self, key: &SerializedPublicKeyString) -> Result<Account> {
        let account = self.state_db.read_handle().get(key).unwrap_or_default();

        Ok(account)
    }

    /// Inserts an account to current state tree.
    pub fn insert_account(
        &mut self,
        key: SerializedPublicKeyString,
        account: Account,
    ) -> Result<()> {
        self.state_db
            .insert(key, account)
            .map_err(|err| StateError::Other(err.to_string()))
    }

    /// Adds multiplpe accounts to current state tree.
    pub fn extend_accounts(&mut self, accounts: Vec<(SerializedPublicKeyString, Account)>) {
        self.state_db.extend(accounts);
    }

    /// Updates an account on the current state tree.
    pub fn update_account(
        &mut self,
        key: SerializedPublicKeyString,
        account: Account,
    ) -> Result<()> {
        self.state_db
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
            .map_err(|err| StateError::Other(err.to_string()))
    }

    pub fn insert_txn_to_mempool(&mut self, txn: Txn) -> Result<()> {
        self.mempool
            .insert(txn)
            .map_err(|err| StateError::Other(err.to_string()))
    }

    pub fn insert_confirmed_txn(&mut self, txn: Txn) -> Result<()> {
        Ok(())
    }

    pub fn remove_txn_from_mempool(&mut self, txn_hash: &TxHashString) -> Result<()> {
        self.mempool
            .remove(txn_hash)
            .map_err(|err| StateError::Other(err.to_string()))?;

        Ok(())
    }
}

impl Clone for NodeState {
    fn clone(&self) -> NodeState {
        NodeState {
            path: self.path.clone(),
            txn_db: self.txn_db.clone(),
            state_db: self.state_db.clone(),
            mempool: self.mempool.clone(),
        }
    }
}

impl From<NodeStateValues> for NodeState {
    fn from(node_state_values: NodeStateValues) -> Self {
        let mut state_db = StateDb::new();
        let mut txn_db = TxnDb::new();

        let mapped_state = node_state_values
            .state
            .into_iter()
            .map(|(key, acc)| (key, acc))
            .collect();

        state_db.extend(mapped_state);

        Self {
            path: PathBuf::new(),
            state_db,
            txn_db,
            mempool: todo!(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NodeStateValues {
    pub txns: HashMap<TxHashString, Txn>,
    pub state: HashMap<SerializedPublicKeyString, Account>,
}

impl From<&NodeState> for NodeStateValues {
    fn from(node_state: &NodeState) -> Self {
        Self {
            txns: HashMap::new(),
            state: node_state.entries(),
        }
    }
}

impl NodeStateValues {
    /// Converts a vector of bytes into a Network State or returns an error if
    /// it's unable to.
    fn from_bytes(data: ByteSlice) -> Result<NodeStateValues> {
        serde_helpers::decode_bytes(data).map_err(|err| StateError::Other(err.to_string()))
    }
}

impl From<ByteVec> for NodeStateValues {
    fn from(data: ByteVec) -> Self {
        Self::from_bytes(&data).unwrap_or_default()
    }
}

impl<'a> From<ByteSlice<'a>> for NodeStateValues {
    fn from(data: ByteSlice) -> Self {
        Self::from_bytes(data).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct NodeStateReadHandle {
    state_handle_factory: StateDbReadHandleFactory,
    mempool_handle_factory: MempoolReadHandleFactory,
}

impl NodeStateReadHandle {
    /// Returns a copy of all values stored within the state trie
    pub fn values(&self) -> HashMap<SerializedPublicKeyString, Account> {
        self.state_handle_factory.handle().entries()
    }

    pub fn mempool_values(&self) -> Vec<Txn> {
        self.mempool_handle_factory.values()
    }
}
