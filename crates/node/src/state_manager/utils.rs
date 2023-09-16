use std::collections::{HashMap, HashSet};

use primitives::Address;
use vrrb_core::account::UpdateArgs;

use crate::state_manager::types::*;

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
                existing_update.code = update.code.clone(); // TODO: Update this to use the most recent value
                if let Some(digests) = update.digests.clone() {
                    if let Some(ref mut existing_digests) = existing_update.digests {
                        existing_digests.extend_all(digests);
                    }
                }
            })
            .or_insert(update);
    }

    consolidated_updates
}
