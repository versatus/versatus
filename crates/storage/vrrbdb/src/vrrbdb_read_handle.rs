use std::collections::HashMap;

use primitives::{Address, TransactionDigest};
use vrrb_core::{account::Account, txn::Txn};

use crate::{StateStoreReadHandleFactory, TransactionStoreReadHandleFactory};

#[derive(Debug, Clone)]
pub struct VrrbDbReadHandle {
    state_store_handle_factory: StateStoreReadHandleFactory,
    transaction_store_handle_factory: TransactionStoreReadHandleFactory,
}

impl VrrbDbReadHandle {
    pub fn new(
        state_store_handle_factory: StateStoreReadHandleFactory,
        transaction_store_handle_factory: TransactionStoreReadHandleFactory,
    ) -> Self {
        Self {
            state_store_handle_factory,
            transaction_store_handle_factory,
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
}
