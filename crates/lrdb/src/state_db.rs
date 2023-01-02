use std::{collections::HashMap, sync::Arc, time::SystemTime};

use crate::result::{LeftRightDbError, Result};
use lr_trie::{InnerTrieWrapper, LeftRightTrie};
use patriecia::db::MemoryDB;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vrrb_core::account::{Account, UpdateArgs};

/// A SHA-256 hash of the miner and validator public key values for a given account
pub type PublicKeysHash = Vec<u8>;

pub type FailedAccountUpdates<'a> = Vec<(&'a PublicKeysHash, Vec<UpdateArgs>, Result<()>)>;

pub struct StateDb<'a> {
    trie: LeftRightTrie<'a, PublicKeysHash, Account, MemoryDB>,
    last_refresh: std::time::SystemTime,
}

// DONE
// TODO: implement serialization and deserialization ops on the wrapper
// TODO: rename generic trie and wrapper simply lr trie
// TODO: replace statedb internals with newly refactored, generic lr trie
//
// TODO
// TODO: refactor other data structures to use bincode instead

impl<'a> Default for StateDb<'a> {
    fn default() -> Self {
        let trie = LeftRightTrie::new(Arc::new(MemoryDB::new(true)));

        Self {
            // TODO: revisit to use utc time
            last_refresh: SystemTime::now(),
            trie,
        }
    }
}

impl<'a> StateDb<'a> {
    /// Returns new, empty instance of StateDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns new ReadHandle to the VrrDb data. As long as the returned value
    /// lives, no write to the database will be committed.
    pub fn read_handle(&self) -> StateDbReadHandle {
        let inner = self.trie.handle();
        StateDbReadHandle { inner }
    }

    /// Commits uncommitted changes to the underlying trie by calling
    /// `publish()` Will wait for EACH ReadHandle to be consumed.
    fn commit_changes(&mut self) {
        self.trie.publish();
        self.last_refresh = SystemTime::now();
    }

    // Maybe initialize is better name for that?
    fn insert_uncommited(&mut self, key: &PublicKeysHash, account: Account) -> Result<()> {
        if account.debits != 0 {
            return Err(LeftRightDbError::Other(
                "cannot insert account with debit".to_string(),
            ));
        }

        if account.nonce != 0 {
            return Err(LeftRightDbError::Other(
                "cannot insert account with nonce bigger than 0".to_string(),
            ));
        }

        self.trie.insert_uncommitted(key, account);

        Ok(())
    }

    /// Inserts new account into StateDb.
    pub fn insert(&mut self, key: &PublicKeysHash, account: Account) -> Result<()> {
        self.insert_uncommited(key, account)?;
        self.commit_changes();
        Ok(())
    }

    // Iterates over provided (PublicKey,DBRecord) pairs, inserting valid ones into
    // the db Returns Option with vec of NOT inserted (PublicKey,DBRecord,e)
    // pairs e being the error which prevented (PublicKey,DBRecord) from being
    // inserted
    fn batch_insert_uncommited(
        &mut self,
        inserts: Vec<(&PublicKeysHash, Account)>,
    ) -> Option<Vec<(&PublicKeysHash, Account, LeftRightDbError)>> {
        let mut failed_inserts: Vec<(&PublicKeysHash, Account, LeftRightDbError)> = vec![];

        inserts.iter().for_each(|item| {
            let (k, v) = item;
            if let Err(e) = self.insert_uncommited(k, v.clone()) {
                failed_inserts.push((k, v.clone(), e));
            }
        });

        if failed_inserts.is_empty() {
            None
        } else {
            Some(failed_inserts)
        }
    }

    /// Inserts a batch of accounts provided in a vector
    ///
    /// Returns None if all inserts were succesfully commited.
    ///
    /// Otherwise returns vector of (key, account_to_be_inserted, error).
    pub fn batch_insert(
        &mut self,
        inserts: Vec<(&PublicKeysHash, Account)>,
    ) -> Option<Vec<(&PublicKeysHash, Account, LeftRightDbError)>> {
        let failed_inserts = self.batch_insert_uncommited(inserts);
        self.commit_changes();
        failed_inserts
    }

    /// Retain returns new StateDb with witch all Accounts that fulfill `filter`
    /// cloned to it.
    pub fn retain<F>(&self, mut filter: F) -> StateDb
    where
        F: FnMut(&Account) -> bool,
    {
        let mut subdb = StateDb::new();

        self.trie.entries().iter().for_each(|(key, value)| {
            let account = value.to_owned();
            if filter(&account) {
                subdb.insert_uncommited(key, account);
            }
        });

        subdb.trie.publish();
        subdb
    }

    /// Returns a number of initialized accounts in the database
    pub fn len(&self) -> usize {
        self.trie.len()
    }

    /// Returns the last refresh time
    pub fn last_refresh(&self) -> SystemTime {
        self.last_refresh
    }

    /// Updates a given account if it exists within the store
    fn update_uncommited(&mut self, key: &PublicKeysHash, update: UpdateArgs) -> Result<()> {
        let account = self
            .read_handle()
            .get(key)
            .map_err(|err| LeftRightDbError::Other(err.to_string()))?;

        let update_result = account
            .update(update)
            .map_err(|err| LeftRightDbError::Other(err.to_string()))?;

        Ok(update_result)
    }

    /// Updates an Account in the database under given PublicKey
    ///
    /// If succesful commits the change. Otherwise returns an error.
    pub fn update(&mut self, key: &PublicKeysHash, update: UpdateArgs) -> Result<()> {
        self.update_uncommited(key, update)?;
        self.commit_changes();
        Ok(())
    }

    // IDEA: Insted of grouping updates by key in advance, we'll just clear oplog
    // from given keys in case error hapens Cannot borrow oplog mutably though
    /// Updates accounts with batch of updates provied in a `updates` vector.
    ///
    /// If there are multiple updates for single PublicKey, those are sorted by
    /// the `nonce` and applied in correct order.
    ///
    /// If at least one update for given account fails, the whole batch for that
    /// `PublicKey` is abandoned.
    ///
    /// All failed batches are returned in vector, with all data - PublicKey for
    /// the account for which the update failed, vector of all updates for that
    /// account, and error that prevented the update.
    pub fn batch_update(
        &mut self,
        mut updates: Vec<(PublicKeysHash, UpdateArgs)>,
    ) -> Option<FailedAccountUpdates> {
        // Store and return all failures as (PublicKey, AllPushedUpdates, Error)
        // This way caller is provided with all info -> They know which accounts were
        // not modified, have a list of all updates to try again And an error
        // thrown so that they can fix it
        let mut failed = FailedAccountUpdates::new();

        // We sort updates by nonce (that's impl of Ord in AccountField)
        // This way all provided updates are used in order (doesn't matter for different
        // accounts, but important for multiple ops on single PubKey)
        updates.sort_by(|a, b| a.1.cmp(&b.1));

        // We'll segregate the batch of updates by key (since it's possible that in
        // provided Vec there is a chance that not every PublicKey is unique)
        let mut update_batches = HashMap::<&PublicKeysHash, Vec<UpdateArgs>>::new();

        updates.iter().for_each(|update| {
            if let Some(vec_of_updates) = update_batches.get_mut(&update.0) {
                vec_of_updates.push(update.1.clone());
            } else {
                update_batches.insert(&update.0, vec![update.1.clone()]);
            }
        });

        // For each PublicKey we try to apply every AccountFieldsUpdate on a copy of
        // current account if event one fails, the whole batch is abandoned with
        // no changes on StateDb when that happens, the key, batch of updates and
        // error are pushed into result vec On success we update the account at
        // given index (PublicKey) We don't need to commit the changes, since we
        // never go back to that key in this function, saving a lot of time (we
        // don't need to wait for all readers to finish)
        update_batches.drain().for_each(|(k, v)| {
            let mut fail: (bool, Result<()>) = (false, Ok(()));
            let mut final_account = Account::new();

            // TODO: refactor the lines below
            let mut account_result = self.read_handle().get(k);

            // match self.read_handle().get(k) {
            match account_result {
                Ok(mut account) => {
                    for update in v.as_slice() {
                        let update_result = account
                            .update(update.clone())
                            .map_err(|err| LeftRightDbError::Other(err.to_string()));

                        if let Err(err) = update_result {
                            fail = (true, Err(err));
                            break;
                        }
                    }
                    final_account = account;
                },
                Err(err) => fail = (true, Err(err)),
            }

            if fail.0 {
                failed.push((k, v, fail.1));
            } else {
                // TODO: implement an update method on underlying lr trie
                self.trie.insert(k, final_account);
                // self.trie.update(k, final_account);
            };
        });

        if failed.len() != updates.len() {
            self.commit_changes();
        };

        if failed.is_empty() {
            return None;
        }

        Some(failed)
    }
}

#[derive(Debug, Clone)]
pub struct StateDbReadHandle {
    inner: InnerTrieWrapper<MemoryDB>,
}

impl StateDbReadHandle {
    /// Returns `Some(Account)` if an account exist under given PublicKey.
    /// Otherwise returns `None`.
    pub fn get(&self, key: &PublicKeysHash) -> Result<Account> {
        self.inner
            .get(key)
            .map_err(|err| LeftRightDbError::Other(err.to_string()))
    }

    /// Get a batch of accounts by providing Vec of PublicKeysHash
    ///
    /// Returns HashMap indexed by PublicKeys and containing either
    /// Some(account) or None if account was not found.
    pub fn batch_get(&self, keys: Vec<PublicKeysHash>) -> HashMap<PublicKeysHash, Option<Account>> {
        let mut accounts = HashMap::new();

        keys.iter().for_each(|key| {
            let value = self.get(key).ok();
            accounts.insert(key.to_owned(), value);
        });

        accounts
    }

    /// Returns a number of initialized accounts in the database
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the information about the StateDb being empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
