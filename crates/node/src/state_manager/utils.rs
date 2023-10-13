use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, RwLock},
};

use block::{Block, BlockHash, ConvergenceBlock, ProposalBlock};
use bulldag::{graph::BullDag, vertex::Vertex};
use ethereum_types::U256;
use events::{Event, EventPublisher};
use mempool::LeftRightMempool;
use primitives::Address;
use storage::vrrbdb::types::*;
use storage::vrrbdb::{StateStoreReadHandle, VrrbDb, VrrbDbReadHandle};
use telemetry::info;
use theater::{ActorId, ActorState};
use vrrb_core::{
    account::{Account, AccountDigests, UpdateArgs},
    claim::Claim,
};

use crate::{NodeError, Result};

/// Converts a HashSet of `StateUpdate`s into a HashSet of `UpdateArgs`s
/// structs.
pub(super) fn get_update_args(updates: HashSet<StateUpdate>) -> HashSet<UpdateArgs> {
    updates.into_iter().map(|update| update.into()).collect()
}

/// Iterates through all `UpdateArgs` structs in a HashSet and consolidates
/// them into a single `UpdateArgs` struct for each address which has
/// activity in a given round.
pub(super) fn consolidate_update_args(
    updates: HashSet<UpdateArgs>,
) -> HashMap<Address, UpdateArgs> {
    let mut consolidated_updates: HashMap<Address, UpdateArgs> = HashMap::new();

    for update in updates.into_iter() {
        let address = update.address.clone();

        consolidated_updates
            .entry(address)
            .and_modify(|existing_update| {
                existing_update.nonce = existing_update.nonce.max(update.nonce);
                existing_update.credits = match (existing_update.credits, update.credits) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, None) => a,
                    (_, b) => b,
                };
                existing_update.debits = match (existing_update.debits, update.debits) {
                    (Some(a), Some(b)) => Some(a + b),
                    (a, None) => a,
                    (_, b) => b,
                };
                existing_update.storage = update.storage.clone(); // TODO: Update this to use the most recent value
                existing_update.package_address = update.package_address.clone(); // TODO: Update this to use the most recent value
                if let Some(digests) = update.digests.clone() {
                    if let Some(ref mut existing_digests) = existing_update.digests {
                        existing_digests.extend_all(digests);
                    } else {
                        existing_update.digests = Some(digests);
                    }
                }
            })
            .or_insert(update);
    }

    consolidated_updates
}
