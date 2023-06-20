use std::collections::HashMap;

use patriecia::Database;
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
pub struct VrrbDbReadHandle<D: Database> {
    state_store_handle_factory: StateStoreReadHandleFactory,
    transaction_store_handle_factory: TransactionStoreReadHandleFactory,
    claim_store_handle_factory: ClaimStoreReadHandleFactory<D>,
}

impl<D: Database> VrrbDbReadHandle<D> {
    pub fn new(
        state_store_handle_factory: StateStoreReadHandleFactory,
        transaction_store_handle_factory: TransactionStoreReadHandleFactory,
        claim_store_handle_factory: ClaimStoreReadHandleFactory<D>,
    ) -> Self {
        Self {
            state_store_handle_factory,
            transaction_store_handle_factory,
            claim_store_handle_factory,
        }
    }

    /// Returns a copy of all values stored within the state trie
    pub fn state_store_values(&self) -> HashMap<Address, Account> {
        self.state_store_handle_factory.handle().entries()
    }

    /// Returns a copy of all values stored within the state trie
    pub fn transaction_store_values(&self) -> HashMap<TransactionDigest, Txn> {
        self.transaction_store_handle_factory.handle().entries()
    }

    pub fn claim_store_values(&self) -> HashMap<NodeId, Claim> {
        self.claim_store_handle_factory.handle().entries()
    }
}
