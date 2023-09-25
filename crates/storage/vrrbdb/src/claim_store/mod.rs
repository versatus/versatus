use std::{path::Path, sync::Arc};

use ethereum_types::U256;
use integral_db::LeftRightTrie;
use patriecia::RootHash;
use sha2::Sha256;
use storage_utils::{Result, StorageError};
use vrrb_core::claim::Claim;

use crate::RocksDbAdapter;

mod claim_store_rh;
pub use claim_store_rh::*;

pub type Claims = Vec<Claim>;
pub type FailedClaimUpdates = Vec<(U256, Claims, Result<()>)>;

#[derive(Debug, Clone)]
pub struct ClaimStore {
    trie: LeftRightTrie<'static, U256, Claim, RocksDbAdapter, Sha256>,
}

impl Default for ClaimStore {
    fn default() -> Self {
        let db_path = storage_utils::get_node_data_dir()
            .unwrap_or_default()
            .join("db")
            .join("claim");

        let db_adapter = RocksDbAdapter::new(db_path, "claims").unwrap_or_default();

        let trie = LeftRightTrie::new(Arc::new(db_adapter));

        Self { trie }
    }
}

impl ClaimStore {
    /// Returns new, empty instance of ClaimDb
    pub fn new(path: &Path) -> Self {
        let path = path.join("claims");
        let db_adapter = RocksDbAdapter::new(path, "claims").unwrap_or_default();
        let trie = LeftRightTrie::new(Arc::new(db_adapter));

        Self { trie }
    }

    /// Returns new ReadHandle to the VrrDb data. As long as the returned value
    /// lives, no write to the database will be committed.
    pub fn read_handle(&self) -> ClaimStoreReadHandle {
        let inner = self.trie.handle();
        ClaimStoreReadHandle::new(inner)
    }

    /// Commits uncommitted changes to the underlying trie by calling
    /// `publish()` Will wait for EACH ReadHandle to be consumed.
    pub fn commit(&mut self) {
        self.trie.publish();
    }

    // Maybe initialize is better name for that?
    fn insert_uncommited(&mut self, claim: Claim) -> Result<()> {
        //        if claim.debits != 0 {
        //            return Err(StorageError::Other(
        //                "cannot insert claim with debit".to_string(),
        //            ));
        //        }
        //
        //        if claim.nonce != 0 {
        //            return Err(StorageError::Other(
        //                "cannot insert claim with nonce bigger than 0".to_string(),
        //            ));
        //        }

        self.trie.insert(claim.hash, claim);

        Ok(())
    }

    /// Inserts new claim into ClaimDb.
    pub fn insert(&mut self, claim: Claim) -> Result<()> {
        self.insert_uncommited(claim)?;
        self.commit();
        Ok(())
    }

    // Iterates over provided (PublicKey,DBRecord) pairs, inserting valid ones into
    // the db Returns Option with vec of NOT inserted (PublicKey,DBRecord,e)
    // pairs e being the error which prevented (PublicKey,DBRecord) from being
    // inserted
    fn batch_insert_uncommited(
        &mut self,
        inserts: Vec<(U256, Claim)>,
    ) -> Option<Vec<(U256, Claim, StorageError)>> {
        let mut failed_inserts: Vec<(U256, Claim, StorageError)> = vec![];

        inserts.iter().for_each(|item| {
            let (k, v) = item;
            if let Err(e) = self.insert_uncommited(v.clone()) {
                failed_inserts.push((k.to_owned(), v.clone(), e));
            }
        });

        if failed_inserts.is_empty() {
            None
        } else {
            Some(failed_inserts)
        }
    }

    /// Inserts a batch of claims provided in a vector
    ///
    /// Returns None if all inserts were succesfully commited.
    ///
    /// Otherwise returns vector of (key, claim_to_be_inserted, error).
    pub fn batch_insert(
        &mut self,
        inserts: Vec<(U256, Claim)>,
    ) -> Option<Vec<(U256, Claim, StorageError)>> {
        let failed_inserts = self.batch_insert_uncommited(inserts);
        self.commit();
        failed_inserts
    }

    /// Retain returns new ClaimDb with which all Claims that fulfill `filter`
    /// cloned to it.
    pub fn retain<F>(&self, _filter: F) -> ClaimStore
    where
        F: FnMut(&Claim) -> bool,
    {
        todo!()
        // let mut subdb = ClaimStore::new(self.);
        //
        // self.trie.entries().iter().for_each(|(key, value)| {
        //     let claim = value.to_owned();
        //     if filter(&claim) {
        //         subdb.insert_uncommited(key.to_string(), claim);
        //     }
        // });
        //
        // subdb.trie.publish();
        // subdb
    }

    /// Returns a number of initialized claims in the database
    pub fn len(&self) -> Result<usize> {
        self.trie
            .len()
            .map_err(|e| StorageError::Other(e.to_string()))
    }

    /// Returns true if the number of initialized claims in the database is
    /// zero.
    pub fn is_empty(&self) -> Result<bool> {
        self.trie
            .is_empty()
            .map_err(|e| StorageError::Other(e.to_string()))
    }

    // TODO: We need to figure out what "updating" a claim means, if anything
    // for now I am leaving these methods out. There will only be inserts,
    // however, updating a claim should include:
    // 1. Stake
    // 2. !Eligible
    //
    /// Updates a given claim if it exists within the store
    //    fn update_uncommited(&mut self, key: NodeId, update: UpdateArgs) ->
    // Result<()> {        let mut claim = self
    //            .read_handle()
    //            .get(&key)
    //            .map_err(|err| StorageError::Other(err.to_string()))?;
    //
    //        claim
    //            .update(update)
    //            .map_err(|err| StorageError::Other(err.to_string()))?;
    //
    //        Ok(())
    //    }
    //
    //    /// Updates an Claim in the database under given PublicKey
    //    ///
    //    /// If succesful commits the change. Otherwise returns an error.
    //    pub fn update(&mut self, key: NodeId, update: UpdateArgs) -> Result<()>
    // {ClaimStoreRead        self.commit_changes();
    //        Ok(())
    //    }
    //
    //    // IDEA: Insted of grouping updates by key in advance, we'll just clear
    // oplog    // from given keys in case error hapens Cannot borrow oplog
    // mutably though    /// Updates claims with batch of updates provied in a
    // `updates` vector.    ///
    //    /// If there are multiple updates for single PublicKey, those are sorted
    // by    /// the `nonce` and applied in correct order.
    //    ///
    //    /// If at least one update for given claim fails, the whole batch for that
    //    /// `PublicKey` is abandoned.
    //    ///
    //    /// All failed batches are returned in vector, with all data - PublicKey
    // for    /// the claim for which the update failed, vector of all updates
    // for that    /// claim, and error that prevented the update.
    //    pub fn batch_update(
    //        &mut self,
    //        mut updates: Vec<(NodeId, UpdateArgs)>,
    //    ) -> Option<FailedClaimUpdates> {
    //        // Store and return all failures as (PublicKey, AllPushedUpdates,
    // Error)        // This way caller is provided with all info -> They know
    // which claims were        // not modified, have a list of all updates to
    // try again And an error        // thrown so that they can fix it
    //        let mut failed = FailedClaimUpdates::new();
    //
    //        // We sort updates by nonce (that's impl of Ord in ClaimField)
    //        // This way all provided updates are used in order (doesn't matter for
    // different        // claims, but important for multiple ops on single
    // PubKey)        updates.sort_by(|a, b| a.1.cmp(&b.1));
    //
    //        // We'll segregate the batch of updates by key (since it's possible
    // that in        // provided Vec there is a chance that not every PublicKey
    // is unique)        let mut update_batches = HashMap::<&NodeId,
    // Vec<UpdateArgs>>::new();
    //
    //        updates.iter().for_each(|update| {
    //            if let Some(vec_of_updates) = update_batches.get_mut(&update.0) {
    //                vec_of_updates.push(update.1.clone());
    //            } else {
    //                update_batches.insert(&update.0, vec![update.1.clone()]);
    //            }
    //        });
    //
    //        // For each PublicKey we try to apply every ClaimFieldsUpdate on a
    // copy of        // current claim if event one fails, the whole batch is
    // abandoned with        // no changes on ClaimDb when that happens, the
    // key, batch of updates and        // error are pushed into result vec On
    // success we update the claim at        // given index (PublicKey) We don't
    // need to commit the changes, since we        // never go back to that key
    // in this function, saving a lot of time (we        // don't need to wait
    // for all readers to finish)        update_batches.drain().for_each(|(k,
    // v)| {            let mut fail: (bool, Result<()>) = (false, Ok(()));
    //            let mut final_claim = Claim::default();
    //
    //            let claim_result = self.read_handle().get(k);
    //
    //            match claim_result {
    //                Ok(mut claim) => {
    //                    for update in v.as_slice() {
    //                        let update_result = claim
    //                            .update(update.clone())
    //                            .map_err(|err|
    // StorageError::Other(err.to_string()));
    //
    //                        if let Err(err) = update_result {
    //                            fail = (true, Err(err));
    //                            break;
    //                        }
    //                    }
    //                    final_claim = claim;
    //                },
    //                Err(err) => fail = (true, Err(err)),
    //            }
    //
    //            if fail.0 {
    //                failed.push((k.to_owned(), v, fail.1));
    //            } else {
    //                // TODO: implement an update method on underlying lr trie
    //                self.trie.insert(k.to_owned(), final_claim);
    //            };
    //        });
    //
    //        if failed.len() != updates.len() {
    //            self.commit_changes();
    //        };
    //
    //        if failed.is_empty() {
    //            return None;
    //        }
    //
    //        Some(failed)
    //    }
    pub fn root_hash(&self) -> Result<RootHash> {
        self.trie
            .root_latest()
            .map_err(|e| StorageError::Other(e.to_string()))
    }

    pub fn extend(&mut self, claims: Vec<(U256, Option<Claim>)>) {
        self.trie.extend(claims)
    }

    pub fn factory(&self) -> ClaimStoreReadHandleFactory {
        let inner = self.trie.factory();

        ClaimStoreReadHandleFactory::new(inner)
    }
}
