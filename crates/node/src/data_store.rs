use std::collections::HashMap;

use block::{Block, ClaimHash};
use primitives::{Address, NodeId, Round};
use storage::{
    storage_utils::StorageError,
    vrrbdb::{Claims, VrrbDb, VrrbDbReadHandle},
};
use vrrb_core::{
    account::Account,
    claim::Claim,
    txn::{TransactionDigest, Txn},
};

use crate::{state_reader::StateReader, Result};

#[async_trait::async_trait]
// NOTE: renamed to DataStore to avoid confusion with StateStore within storage crate
pub trait DataStore<S: StateReader> {
    type Error;

    fn state_reader(&self) -> S;
}
