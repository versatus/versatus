use std::collections::HashMap;

use integral_db::{JellyfishMerkleTreeWrapper, ReadHandleFactory};
use patriecia::{JellyfishMerkleTree, Version};
use primitives::NodeId;
use sha2::Sha256;
use storage_utils::{Result, StorageError};
use vrrb_core::claim::Claim;

use crate::RocksDbAdapter;

#[derive(Debug, Clone)]
pub struct ClaimStoreReadHandle {
    inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, Sha256>,
}

impl ClaimStoreReadHandle {
    pub fn new(inner: JellyfishMerkleTreeWrapper<RocksDbAdapter, Sha256>) -> Self {
        Self { inner }
    }

    /// Returns `Some(Claim)` if an account exist under given PublicKey.
    /// Otherwise returns `None`.
    pub fn get(&self, key: &NodeId, version: Version) -> Result<Claim> {
        self.inner
            .get(key, version)
            .map_err(|err| StorageError::Other(err.to_string()))
    }

    /// Get a batch of claims by providing Vec of PublicKeysHash
    ///
    /// Returns HashMap indexed by PublicKeys and containing either
    /// Some(account) or None if account was not found.
    pub fn batch_get(&self, keys: Vec<NodeId>, version: Version) -> HashMap<NodeId, Option<Claim>> {
        let mut claims = HashMap::new();

        keys.iter().for_each(|key| {
            let value = self.get(key, version).ok();
            claims.insert(key.to_owned(), value);
        });

        claims
    }

    pub fn entries(&self) -> HashMap<NodeId, Claim> {
        // TODO: revisit and refactor into inner wrapper
        self.inner
            .iter(self.inner.version())
            .unwrap()
            .filter_map(|item| {
                if let Ok((_, claim)) = item {
                    if let Ok(claim) = bincode::deserialize::<Claim>(&claim) {
                        return Some((claim.node_id.clone(), claim)); // The default is a place holder, this is broken atm
                                                                     // since we cannot get the NodeId from the Claim nor the KeyHash
                    }
                }
                None
            })
            .collect()
    }

    /// Returns a number of initialized claims in the database
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the information about the ClaimDb being empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ClaimStoreReadHandleFactory {
    inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, Sha256>>,
}

impl ClaimStoreReadHandleFactory {
    pub fn new(inner: ReadHandleFactory<JellyfishMerkleTree<RocksDbAdapter, Sha256>>) -> Self {
        Self { inner }
    }

    pub fn handle(&self) -> ClaimStoreReadHandle {
        let handle = self
            .inner
            .handle()
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        let inner = JellyfishMerkleTreeWrapper::new(handle);

        ClaimStoreReadHandle { inner }
    }
}
