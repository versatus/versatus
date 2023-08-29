use std::collections::HashMap;

use patriecia::{KeyHash, Version};
use primitives::{Address, NodeId};
use vrrb_core::{
    account::Account,
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

use crate::{
    ClaimStoreReadHandleFactory, StateStoreReadHandleFactory, TransactionStoreReadHandleFactory,
};

#[derive(Debug, Clone)]
pub struct VrrbDbReadHandle {
    state_store_handle_factory: StateStoreReadHandleFactory,
    transaction_store_handle_factory: TransactionStoreReadHandleFactory,
    claim_store_handle_factory: ClaimStoreReadHandleFactory,
}

impl VrrbDbReadHandle {
    pub fn new(
        state_store_handle_factory: StateStoreReadHandleFactory,
        transaction_store_handle_factory: TransactionStoreReadHandleFactory,
        claim_store_handle_factory: ClaimStoreReadHandleFactory,
    ) -> Self {
        Self {
            state_store_handle_factory,
            transaction_store_handle_factory,
            claim_store_handle_factory,
        }
    }
    // TODO: rewrite these to get start at the first key available and the latest version
    /// Returns a copy of all values stored within the state trie
    pub fn state_store_values(&self) -> HashMap<Address, Account> {
        self.state_store_handle_factory.handle().entries()
    }

    // TODO: rewrite these to get start at the first key available and the latest version
    /// Returns a copy of all values stored within the state trie
    pub fn transaction_store_values(&self) -> HashMap<TransactionDigest, Txn> {
        self.transaction_store_handle_factory.handle().entries()
    }

    // TODO: rewrite these to get start at the first key available and the latest version
    /// Returns a copy of all values stored within the state trie
    pub fn claim_store_values(&self) -> HashMap<NodeId, Claim> {
        self.claim_store_handle_factory.handle().entries()
    }
}
