use std::{collections::HashMap, path::Path, sync::Arc};

use integral_db::LeftRightTrie;
use patriecia::RootHash;
use primitives::Address;
use sha2::Sha256;
use storage_utils::{Result, StorageError};
use vrrb_core::account::{Account, UpdateArgs};

use crate::RocksDbAdapter;

mod state_store_rh;
pub use state_store_rh::*;

pub type Accounts = Vec<Account>;
pub type FailedAccountUpdates = Vec<(Address, Vec<UpdateArgs>, Result<()>)>;

#[derive(Debug, Clone)]
pub struct StateStore {
    trie: LeftRightTrie<'static, Address, Account, RocksDbAdapter, Sha256>,
}

impl Default for StateStore {
    fn default() -> Self {
        let db_path = storage_utils::get_node_data_dir()
            .unwrap_or_default()
            .join("db")
            .join("state");

        let db_adapter = RocksDbAdapter::new(db_path, "state").unwrap_or_default();

        let trie = LeftRightTrie::new(Arc::new(db_adapter));

        Self { trie }
    }
}

impl StateStore {
    /// Returns new, empty instance of StateDb

    pub fn new(path: &Path) -> Self {
        let path = path.join("state");
        let db_adapter = RocksDbAdapter::new(path, "state").unwrap_or_default();
        let trie = LeftRightTrie::new(Arc::new(db_adapter));

        Self { trie }
    }

    /// Returns new ReadHandle to the VrrDb data. As long as the returned value
    /// lives, no write to the database will be committed.
    pub fn read_handle(&self) -> StateStoreReadHandle {
        let inner = self.trie.handle();
        StateStoreReadHandle::new(inner)
    }

    pub fn commit(&mut self) {
        self.trie.publish();
    }

    pub fn get_account(&self, key: &Address) -> Result<Account> {
        let read_handle = self.read_handle();
        read_handle.get(key)
    }

    /// Commits uncommitted changes to the underlying trie by calling
    /// `publish()` Will wait for EACH ReadHandle to be consumed.
    fn commit_changes(&mut self) {
        self.trie.publish();
    }

    // Maybe initialize is better name for that?
    fn insert_uncommited(&mut self, key: Address, account: Account) -> Result<()> {
        if account.debits() != 0 {
            return Err(StorageError::Other(
                "cannot insert account with debit".to_string(),
            ));
        }

        if account.nonce() != 0 {
            return Err(StorageError::Other(
                "cannot insert account with nonce bigger than 0".to_string(),
            ));
        }

        self.trie.insert(key, account);

        Ok(())
    }

    /// Inserts new account into StateDb.
    pub fn insert(&mut self, key: Address, account: Account) -> Result<()> {
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
        inserts: Vec<(Address, Account)>,
    ) -> Option<Vec<(Address, Account, StorageError)>> {
        let mut failed_inserts: Vec<(Address, Account, StorageError)> = vec![];

        inserts.iter().for_each(|item| {
            let (k, v) = item;
            if let Err(e) = self.insert_uncommited(k.to_owned(), v.clone()) {
                failed_inserts.push((k.to_owned(), v.clone(), e));
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
        inserts: Vec<(Address, Account)>,
    ) -> Option<Vec<(Address, Account, StorageError)>> {
        let failed_inserts = self.batch_insert_uncommited(inserts);
        self.commit_changes();
        failed_inserts
    }

    /// Retain returns new StateDb with which all Accounts that fulfill `filter`
    /// cloned to it.
    pub fn retain<F>(&self, _filter: F) -> StateStore
    where
        F: FnMut(&Account) -> bool,
    {
        todo!()
        // let mut subdb = StateStore::new(self.);
        //
        // self.trie.entries().iter().for_each(|(key, value)| {
        //     let account = value.to_owned();
        //     if filter(&account) {
        //         subdb.insert_uncommited(key.to_string(), account);
        //     }
        // });
        //
        // subdb.trie.publish();
        // subdb
    }

    /// Returns a number of initialized accounts in the database
    pub fn len(&self) -> Result<usize> {
        self.trie
            .len()
            .map_err(|e| StorageError::Other(e.to_string()))
    }

    /// Returns true if the number of initialized accounts in the database is
    /// zero.
    pub fn is_empty(&self) -> Result<bool> {
        self.trie
            .is_empty()
            .map_err(|e| StorageError::Other(e.to_string()))
    }

    /// Updates a given account if it exists within the store
    pub fn update_uncommited(&mut self, key: Address, update: UpdateArgs) -> Result<()> {
        let mut account = self
            .read_handle()
            .get(&key)
            .map_err(|err| StorageError::Other(err.to_string()))?;

        account
            .update(update)
            .map_err(|err| StorageError::Other(err.to_string()))?;

        self.trie.update(key, account.clone());

        Ok(())
    }

    /// Updates an Account in the database under given PublicKey
    ///
    /// If succesful commits the change. Otherwise returns an error.
    pub fn update(&mut self, update: UpdateArgs) -> Result<()> {
        let key = update.address.clone();
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
        mut updates: Vec<(Address, UpdateArgs)>,
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
        let mut update_batches = HashMap::<&Address, Vec<UpdateArgs>>::new();

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
            let mut final_account = Account::default();

            let account_result = self.read_handle().get(k);

            match account_result {
                Ok(mut account) => {
                    for update in v.as_slice() {
                        let update_result = account
                            .update(update.clone())
                            .map_err(|err| StorageError::Other(err.to_string()));

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
                failed.push((k.to_owned(), v, fail.1));
            } else {
                // TODO: implement an update method on underlying lr trie
                self.trie.insert(k.to_owned(), final_account);
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

    pub fn root_hash(&self) -> Result<RootHash> {
        self.trie
            .root_latest()
            .map_err(|e| StorageError::Other(e.to_string()))
    }

    pub fn extend(&mut self, accounts: Vec<(Address, Option<Account>)>) {
        self.trie.extend(accounts)
    }

    pub fn factory(&self) -> StateStoreReadHandleFactory {
        let inner = self.trie.factory();

        StateStoreReadHandleFactory::new(inner)
    }
}
