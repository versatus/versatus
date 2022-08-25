use std::{cmp::Ordering, collections::HashMap, hash::Hash, time::SystemTime};

use lr_trie::db::Database;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type Nonce = u32;

/// Stores information about given account.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Account {
    pub hash: String,
    pub nonce: Nonce,
    pub credits: u64,
    pub debits: u64,
    pub storage: Option<String>,
    pub code: Option<String>,
}

impl Account {
    /// Returns new, empty account.
    ///
    /// Examples:
    /// ```
    /// use lrdb::lr_db::Account;
    ///
    /// let account = Account::new();
    /// ```
    pub fn new() -> Account {
        let nonce = 0u32;
        let credits = 0u64;
        let debits = 0u64;
        let storage = None;
        let code = None;

        let mut hasher = Sha256::new();
        hasher.update(&nonce.to_be_bytes());
        hasher.update(&credits.to_be_bytes());
        hasher.update(&debits.to_be_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Account {
            hash,
            nonce,
            credits,
            debits,
            storage,
            code,
        }
    }

    /// Modifies accounts hash, recalculating it using account's fields.
    fn update_hash(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(self.nonce.to_be_bytes());
        hasher.update(self.credits.to_be_bytes());
        hasher.update(self.debits.to_be_bytes());

        if let Some(storage) = &self.storage {
            hasher.update(storage.as_bytes());
        }

        if let Some(code) = &self.code {
            hasher.update(code.as_bytes());
        }
        self.hash = format!("{:x}", hasher.finalize());
    }

    // TODO: do those safely
    // Should we rollback credits and debits on overflow?
    // e.g.
    // self.credits -= self.debits;
    // self.debits -= self.debits;
    //
    // This way overall balance stays the same
    // But the numbers are fine
    // This may be a problem since even though u64 (or whatever we end up using) are big
    // Imagining some trading account, at one point it could fill up (with thousands of transactions per day)

    /// Updates single field in account struct without updating it's hash.
    /// Unsafe to use alone (hash should be recalculated).
    /// Used only in batch updates to improve speed by reducing unnecesary hash calculations.
    /// Returns error if update fails.
    fn update_single_field_no_hash(&mut self, value: AccountField) -> Result<(), VrrbDbError> {
        match value {
            AccountField::Credits(credits) => match self.credits.checked_add(credits) {
                Some(new_amount) => self.credits = new_amount,
                None => return Err(VrrbDbError::UpdateFailed(value)),
            },
            AccountField::Debits(debits) => match self.debits.checked_add(debits) {
                Some(new_amount) => {
                    if self.credits >= new_amount {
                        self.debits = new_amount
                    } else {
                        return Err(VrrbDbError::UpdateFailed(value));
                    }
                }
                None => return Err(VrrbDbError::UpdateFailed(value)),
            },

            // Should the storage be impossible to delete?
            AccountField::Storage(storage) => {
                self.storage = storage;
            }

            // Should the code be impossible to delete?
            AccountField::Code(code) => {
                self.code = code;
            }
        }
        Ok(())
    }

    /// Updates single field of the struct. Doesn't update the nonce.
    /// Before trying to update account in database with this account, nonce should be bumped.
    /// Finaly recalculates and updates the hash. Might return an error.
    ///
    /// # Arguments:
    /// * `update` - An AccountField enum specifying which field update (with what value)
    ///
    /// # Examples:
    /// ```
    /// use lrdb::lr_db::{Account, AccountField};
    /// let mut account = Account::new();
    /// account.update_field(AccountField::Credits(300));
    /// account.bump_nonce();
    ///
    /// assert_eq!(account.credits, 300);
    /// ```
    pub fn update_field(&mut self, update: AccountField) -> Result<(), VrrbDbError> {
        let res = self.update_single_field_no_hash(update);
        self.update_hash();
        res
    }

    /// Updates all fields of the account struct accoring to supplied AccountFieldsUpdate struct.
    /// Requires provided nonce (update's nonce) to be exactly one number higher than accounts nonce.
    /// Recalculates hash. Might return an error.
    ///
    /// # Arguments:
    /// * `update` - An AccountFieldsUpdate struct containing instructions to update each field of the account struct.
    ///
    /// # Example:
    /// ```
    /// use lrdb::lr_db::{Account, AccountFieldsUpdate};
    ///
    /// let mut account = Account::new();
    /// let update = AccountFieldsUpdate{
    ///     nonce: account.nonce + 1,
    ///     credits: Some(32),
    ///     debits: None,
    ///     storage: None,
    ///     code: Some(Some("Some code".to_string()))
    /// };
    ///
    /// account.update(update);
    ///
    /// assert_eq!(account.credits, 32);
    /// assert_eq!(account.code, Some("Some code".to_string()));
    /// ```
    pub fn update(&mut self, update: AccountFieldsUpdate) -> Result<(), VrrbDbError> {
        if self.nonce + 1 != update.nonce {
            return Err(VrrbDbError::InvalidUpdateNonce(self.nonce, update.nonce));
        }
        if let Some(credits_update) = update.credits {
            self.update_single_field_no_hash(AccountField::Credits(credits_update))?;
        }
        if let Some(debits_update) = update.debits {
            self.update_single_field_no_hash(AccountField::Debits(debits_update))?;
        }
        if let Some(code_update) = update.code {
            self.update_single_field_no_hash(AccountField::Code(code_update))?;
        }
        if let Some(storage_update) = update.storage {
            self.update_single_field_no_hash(AccountField::Storage(storage_update))?;
        }

        self.bump_nonce();
        self.update_hash();
        Ok(())
    }

    pub fn bump_nonce(&mut self) {
        self.nonce += 1;
    }
}

/// Enum containing options for updates - used to update value of single field in account struct.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AccountField {
    Credits(u64),
    Debits(u64),
    Storage(Option<String>),
    Code(Option<String>),
}

/// Struct representing the LeftRight Database.
///
/// `ReadHandleFactory` provides a way of creating new ReadHandles to the database.
///
/// `WriteHandles` provides a way to gain write access to the database.
/// `last_refresh` denotes the lastest `refresh` of the database.
#[allow(dead_code)]
pub struct LeftRightDatabase<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone + Eq + evmap::ShallowCopy,
{
    r: evmap::ReadHandleFactory<K, V, ()>,
    w: evmap::WriteHandle<K, V, ()>,
    last_refresh: std::time::SystemTime,
}

/// NOTE: Boxing the Account so it's `ShallowCopy`
/// ShallowCopy is unsafe. To work properly it requires that type is never modified.
#[allow(dead_code)]
/// Type wrapping LeftRightDatabase with non-generic arguments used in this implementation.
pub type VrrbDb = LeftRightDatabase<PublicKey, Box<Account>>;

/// Struct representing the desired updates to be applied to account.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AccountFieldsUpdate {
    pub nonce: u32,
    pub credits: Option<u64>,
    pub debits: Option<u64>,
    pub storage: Option<Option<String>>,
    pub code: Option<Option<String>>,
}

impl Default for AccountFieldsUpdate {
    fn default() -> Self {
        AccountFieldsUpdate {
            nonce: 0,
            credits: None,
            debits: None,
            storage: None,
            code: None,
        }
    }
}

// The AccountFieldsUpdate will be compared by `nonce`. This way the updates can be properly scheduled.
impl Ord for AccountFieldsUpdate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nonce.cmp(&other.nonce)
    }
}

impl PartialOrd for AccountFieldsUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct VrrbDbReadHandle {
    rh: evmap::ReadHandle<PublicKey, Box<Account>>,
}

impl VrrbDbReadHandle {
    /// Returns `Some(Account)` if an account exist under given PublicKey.
    /// Otherwise returns `None`.
    ///
    /// Arguments:
    ///
    /// * `key` - PublicKey indexing the account
    ///
    /// Examples:
    ///
    /// ```
    ///  use lrdb::lr_db::{VrrbDb, Account, AccountField};
    ///  use secp256k1::PublicKey;
    ///  use rand::{rngs::StdRng, SeedableRng};
    ///  use secp256k1::generate_keypair;
    ///   
    ///  let (_, key) = generate_keypair(&mut StdRng::from_entropy());
    ///  let mut vdb = VrrbDb::new();
    ///    
    ///  let mut account = Account::new();
    ///  account.update_field(AccountField::Credits(123));
    ///
    ///  vdb.insert(key, account);
    ///
    ///  if let Some(account) = vdb.read_handle().get(key) {
    ///     assert_eq!(account.credits, 123);
    ///  }
    /// ```
    pub fn get(&self, key: PublicKey) -> Option<Account> {
        self.rh.get_and(&key, |x| return *x[0].clone())
    }

    /// Get a batch of accounts by providing Vec of PublicKeys
    ///
    /// Returns HashMap indexed by PublicKeys and containing either Some(account) or None if account was not found.
    ///
    /// Arguments:
    ///
    /// * `keys` - Vector of public keys for accounts to be fetched.
    ///
    /// Example:
    ///
    /// ```
    ///  use lrdb::lr_db::{VrrbDb, Account, AccountField};
    ///  use secp256k1::PublicKey;
    ///  use rand::{rngs::StdRng, SeedableRng};
    ///  use secp256k1::generate_keypair;
    ///   
    ///  let (_, key) = generate_keypair(&mut StdRng::from_entropy());
    ///  let (_, key2) = generate_keypair(&mut StdRng::from_entropy());
    ///  let mut vdb = VrrbDb::new();
    ///      
    ///  let mut account = Account::new();
    ///  account.update_field(AccountField::Credits(123));
    ///
    ///  let mut account2 = Account::new();
    ///  account2.update_field(AccountField::Credits(257));
    ///   
    ///  vdb.batch_insert(vec![(key, account), (key2,account2.clone())]);
    ///
    ///  let result = vdb.read_handle().batch_get(vec![key, key2]);
    ///  
    ///  assert_eq!(result[&key2].clone().unwrap().credits, 257);
    ///     
    /// ```
    pub fn batch_get(&self, keys: Vec<PublicKey>) -> HashMap<PublicKey, Option<Account>> {
        let mut accounts = HashMap::<PublicKey, Option<Account>>::new();
        keys.iter().for_each(|k| {
            accounts.insert(k.clone(), self.get(*k));
        });
        accounts
    }

    /// Returns a number of initialized accounts in the database
    pub fn len(&self) -> usize {
        self.rh.len()
    }
}

impl VrrbDb {
    /// Returns new, empty account.
    ///
    /// Examples:
    /// ```
    /// use lrdb::lr_db::VrrbDb;
    ///
    /// let vdb = VrrbDb::new();
    /// ```
    pub fn new() -> Self {
        let (vrrbdb_reader, mut vrrbdb_writer) = evmap::new();
        // This is required to set up oplog
        // Otherwise there's no way to keep track of already inserted keys (before refresh)
        vrrbdb_writer.refresh();
        Self {
            r: vrrbdb_reader.factory(),
            w: vrrbdb_writer,
            last_refresh: SystemTime::now(),
        }
    }

    /// Returns new ReadHandle to the VrrDb data. As long as the returned value lives,
    /// no write to the database will be committed.
    pub fn read_handle(&self) -> VrrbDbReadHandle {
        VrrbDbReadHandle {
            rh: self.r.handle(),
        }
    }

    // Commits uncommited changes
    /// Commits the pending changes to the underlying evmap by calling `refresh()`
    /// Will wait for EACH ReadHandle to be consumed.
    fn commit_changes(&mut self) {
        self.w.refresh();
        self.last_refresh = SystemTime::now();
    }

    // Maybe initialize is better name for that?
    fn insert_uncommited(
        &mut self,
        pubkey: PublicKey,
        account: Account,
    ) -> Result<(), VrrbDbError> {
        // Cannot insert new account with debit
        if account.debits != 0 {
            return Err(VrrbDbError::InitWithDebit);
        }

        // Cannot insert account with nonce bigger than 0
        if account.nonce != 0 {
            return Err(VrrbDbError::InitWithNonce);
        }

        // Check oplog for uncommited inserts for given key
        // That should only be possible if the same PublicKey is used in batch_insert
        let mut found = false;
        self.w.pending().iter().for_each(|op| {
            if let evmap::Operation::Add::<PublicKey, Box<Account>>(key, _) = op {
                if pubkey == *key {
                    found = true;
                }
            }
        });
        match self.r.handle().contains_key(&pubkey) || found {
            true => return Err(VrrbDbError::RecordExists),
            false => {
                self.w.insert(pubkey, Box::new(account));
                return Ok(());
            }
        }
    }

    /// Inserts new account into VrrbDb.
    ///
    /// Arguments:
    /// * `pubkey` - A PublicKey of account to be inserted
    /// * `account` - An account struct containing account's info. For valid insertion
    /// account's debit and nonce should be default value (0)
    ///
    /// Examples:
    ///
    /// Basic usage:
    ///
    /// ```
    /// use lrdb::lr_db::{Account, VrrbDb, VrrbDbError};
    /// use secp256k1::PublicKey;
    /// use rand::{rngs::StdRng, SeedableRng};
    /// use secp256k1::generate_keypair;
    ///   
    /// let (_, key) = generate_keypair(&mut StdRng::from_entropy());
    ///
    /// let account = Account::new();
    /// let mut vdb = VrrbDb::new();
    ///
    /// let added = vdb.insert(key, account);
    /// assert_eq!(added, Ok(()));
    /// ```
    ///
    /// Failed inserts:
    ///
    /// ```
    /// use lrdb::lr_db::{Account, VrrbDb, VrrbDbError};
    /// use secp256k1::{PublicKey};
    /// use rand::{rngs::StdRng, SeedableRng};
    /// use secp256k1::generate_keypair;
    ///   
    /// let (_, key) = generate_keypair(&mut StdRng::from_entropy());
    /// let mut account = Account::new();
    ///
    /// // That will fail, since nonce should be 0
    /// account.nonce = 10;
    /// let mut vdb = VrrbDb::new();
    /// let mut added = vdb.insert(key, account.clone());
    ///
    /// assert_eq!(added, Err(VrrbDbError::InitWithNonce));
    ///  
    /// account.nonce = 0;
    /// account.debits = 10;
    ///
    /// // This will fail since the debit should be 0
    /// added = vdb.insert(key,account);
    ///
    /// assert_eq!(added, Err(VrrbDbError::InitWithDebit));
    /// ```
    pub fn insert(&mut self, pubkey: PublicKey, account: Account) -> Result<(), VrrbDbError> {
        self.insert_uncommited(pubkey, account)?;
        self.commit_changes();
        Ok(())
    }

    // Iterates over provided (PublicKey,DBRecord) pairs, inserting valid ones into the db
    // Returns Option with vec of NOT inserted (PublicKey,DBRecord,e) pairs
    // e being the error which prevented (PublicKey,DBRecord) from being inserted
    fn batch_insert_uncommited(
        &mut self,
        inserts: Vec<(PublicKey, Account)>,
    ) -> Option<Vec<(PublicKey, Account, VrrbDbError)>> {
        let mut failed_inserts: Vec<(PublicKey, Account, VrrbDbError)> = vec![];

        inserts.iter().for_each(|item| {
            let (k, v) = item;
            if let Err(e) = self.insert_uncommited(k.clone(), v.clone()) {
                failed_inserts.push((k.clone(), v.clone(), e));
            }
        });

        if failed_inserts.is_empty() {
            return None;
        } else {
            return Some(failed_inserts);
        }
    }

    /// Inserts a batch of accounts provided in a vector
    ///
    /// Returns None if all inserts were succesfully commited.
    ///
    /// Otherwise returns vector of (key, account_to_be_inserted, error).
    ///
    /// Arguments:
    ///
    /// * `inserts` - Vec<(PublicKey, Account)> - Vector of tuples - containing target (unique) PublicKey and Account struct to be inserted
    ///
    /// Examples:
    /// ```
    /// use lrdb::lr_db::{Account, VrrbDb, AccountField};
    /// use secp256k1::PublicKey;
    ///
    /// let mut vrrbdb = VrrbDb::new();
    /// use rand::{rngs::StdRng, SeedableRng};
    /// use secp256k1::generate_keypair;
    ///
    /// let mut rng = StdRng::from_entropy();
    /// let (_, key) = generate_keypair(&mut rng);
    /// let (_, key1) = generate_keypair(&mut rng);
    /// let (_, key2) = generate_keypair(&mut rng);
    ///
    /// let mut account1 = Account::new();
    /// account1.update_field(AccountField::Credits(100));
    ///  
    /// let mut account2 = Account::new();
    /// account2.update_field(AccountField::Credits(237));
    ///    
    /// let mut account3 = Account::new();
    /// account3.update_field(AccountField::Credits(500));
    ///
    /// vrrbdb.batch_insert(vec![(key, account1), (key1, account2), (key2, account3)]);
    ///
    /// if let Some(account) = vrrbdb.read_handle().get(key1) {
    ///     assert_eq!(account.credits, 237);
    /// }
    /// ```
    pub fn batch_insert(
        &mut self,
        inserts: Vec<(PublicKey, Account)>,
    ) -> Option<Vec<(PublicKey, Account, VrrbDbError)>> {
        let failed_inserts = self.batch_insert_uncommited(inserts);
        self.commit_changes();
        failed_inserts
    }

    /// Retain returns new VrrbDb with witch all Accounts that fulfill `filter` cloned to it.
    ///
    /// Arguments:
    ///
    /// * `filter` - closure specifying filter for cloned accounts
    ///
    /// Examples:
    ///    ```
    ///    use lrdb::lr_db::{VrrbDb, Account, AccountField};
    ///    use secp256k1::PublicKey;
    ///    use std::str::FromStr;
    ///    use rand::{rngs::StdRng, SeedableRng};
    ///    use secp256k1::generate_keypair;
    ///   
    ///    let (_, key) = generate_keypair(&mut StdRng::from_entropy());
    ///    let (_, key1) = generate_keypair(&mut StdRng::from_entropy());
    ///    let (_, key2) = generate_keypair(&mut StdRng::from_entropy());
    ///    let (_, key3) = generate_keypair(&mut StdRng::from_entropy());
    ///    let mut vdb = VrrbDb::new();
    ///    
    ///
    ///    let mut account = Account::new();
    ///    account.update_field(AccountField::Credits(123));
    ///
    ///    let mut account1 = Account::new();
    ///    account1.update_field(AccountField::Credits(250));
    ///
    ///    let mut account2 = Account::new();
    ///    account2.update_field(AccountField::Credits(300));
    ///
    ///    let mut account3 = Account::new();
    ///    account3.update_field(AccountField::Credits(500));
    ///
    ///    vdb.batch_insert(vec![(key, account), (key1, account1), (key2, account2.clone()), (key3, account3)]);
    ///
    ///    let filtered = vdb.retain(|acc| {acc.credits >= 300 && acc.credits < 500});
    ///     
    ///    assert_eq!(filtered.len(), 1);
    ///
    /// ```
    pub fn retain<F>(&self, mut filter: F) -> VrrbDb
    where
        F: FnMut(&Account) -> bool,
    {
        let mut subdb = VrrbDb::new();
        self.r.handle().for_each(|key, val| {
            let account = *val[0].clone();
            if filter(&account) {
                subdb.w.insert(*key, Box::new(account));
            }
        });
        subdb.w.refresh();
        subdb
    }

    /// Returns a number of initialized accounts in the database
    pub fn len(&self) -> usize {
        self.w.len()
    }

    // Returns the last refresh time
    pub fn last_refresh(&self) -> SystemTime {
        self.last_refresh
    }

    // If possible, updates the account. Otherwise returns an error
    fn update_uncommited(
        &mut self,
        key: PublicKey,
        update: AccountFieldsUpdate,
    ) -> Result<(), VrrbDbError> {
        match self.read_handle().get(key) {
            Some(mut account) => {
                account.update(update)?;
                self.w.update(key, Box::new(account));

                return Ok(());
            }
            None => return Err(VrrbDbError::AccountDoesntExist(key)),
        }
    }

    /// Updates an Account in the database under given PublicKey
    ///
    /// If succesful commits the change. Otherwise returns an error.
    ///
    /// Arguments:
    ///
    /// `key` - PublicKey of the account to update
    ///
    /// `update` - AccountFieldUpdate struct with update values for given fields.
    ///
    /// Example:
    ///
    /// ```
    ///    use lrdb::lr_db::{VrrbDb, Account, AccountField, AccountFieldsUpdate};
    ///    use secp256k1::PublicKey;
    ///    use rand::{rngs::StdRng, SeedableRng};
    ///    use secp256k1::generate_keypair;
    ///    
    ///    let (_,key) = generate_keypair(&mut StdRng::from_entropy());
    ///    let mut vdb = VrrbDb::new();
    ///      
    ///    let mut account = Account::new();
    ///    account.update_field(AccountField::Credits(123));
    ///     
    ///    let update = AccountFieldsUpdate {
    ///        nonce: 1,
    ///        credits: Some(100),
    ///        debits: Some(50),
    ///        storage: None,
    ///        code: None,
    ///    };
    ///    
    ///    vdb.update(key, update);
    ///
    ///    if let Some(acc) = vdb.read_handle().get(key) {
    ///       assert_eq!(acc.credits,100);
    ///       assert_eq!(acc.debits, 50);    
    ///    }
    ///   
    /// ```
    pub fn update(
        &mut self,
        key: PublicKey,
        update: AccountFieldsUpdate,
    ) -> Result<(), VrrbDbError> {
        self.update_uncommited(key, update)?;
        self.commit_changes();
        Ok(())
    }

    // IDEA: Insted of grouping updates by key in advance, we'll just clear oplog from given keys in case error hapens
    // Cannot borrow oplog mutably though
    /// Updates accounts with batch of updates provied in a `updates` vector.
    ///
    /// If there are multiple updates for single PublicKey, those are sorted by the `nonce` and applied in correct order.
    ///
    /// If at least one update for given account fails, the whole batch for that `PublicKey` is abandoned.
    ///
    /// All failed batches are returned in vector, with all data - PublicKey for the account for which the update failed, vector of all updates for that account, and error that prevented the update.
    ///
    /// Arguments:
    ///
    /// * `updates` - vector of tuples containing PublicKey for account and AccountFieldsUpdate struct with update values.
    ///
    /// Examples:
    ///
    /// ```
    ///    use lrdb::lr_db::{VrrbDb, Account, AccountField, AccountFieldsUpdate};
    ///    use secp256k1::PublicKey;
    ///    use rand::{rngs::StdRng, SeedableRng};
    ///    use secp256k1::generate_keypair;
    ///
    ///    let mut vdb = VrrbDb::new();

    ///    let account = Account::new();

    ///    let (_,key) = generate_keypair(&mut StdRng::from_entropy());
    ///    let (_,key1) = generate_keypair(&mut StdRng::from_entropy());
    ///    let (_,key2) = generate_keypair(&mut StdRng::from_entropy());
    ///    let (_,key3) = generate_keypair(&mut StdRng::from_entropy());
    ///
    ///    let updates = vec![
    ///        AccountFieldsUpdate {
    ///            nonce: account.nonce + 1,
    ///            credits: Some(1230),
    ///            debits: Some(10),
    ///            storage: Some(Some("Some storage".to_string())),
    ///            ..Default::default()
    ///        },
    ///        AccountFieldsUpdate {
    ///            credits: Some(100),
    ///            debits: Some(300),
    ///            nonce: account.nonce + 2,
    ///            ..Default::default()
    ///        },
    ///    ];

    ///    let account1 = Account::new();
    ///    let update1 = AccountFieldsUpdate {
    ///        nonce: account1.nonce + 1,
    ///        credits: Some(250),
    ///        ..Default::default()
    ///    };

    ///    let account2 = Account::new();
    ///    let update2 = AccountFieldsUpdate {
    ///        nonce: account2.nonce + 1,
    ///        credits: Some(300),
    ///        debits: Some(250),
    ///        code: Some(Some("test".to_string())),
    ///        ..Default::default()
    ///    };

    ///    let mut account3 = Account::new();
    ///    let updates3 = vec![
    ///        AccountFieldsUpdate {
    ///            nonce: account3.nonce + 1,
    ///            credits: Some(500),
    ///            debits: Some(500),
    ///            ..Default::default()
    ///        },
    ///        AccountFieldsUpdate {
    ///            nonce: account3.nonce + 2,
    ///            credits: Some(90),
    ///            ..Default::default()
    ///        },
    ///    ];

    ///    if let Some(failed) = vdb.batch_insert(vec![
    ///        (key, account.clone()),
    ///        (key1, account1.clone()),
    ///        (key2, account2.clone()),
    ///        (key3, account3.clone()),
    ///    ]) {
    ///        failed.iter().for_each(|(k, v, e)| {
    ///            println!("{:?}", e);
    ///        });
    ///    };

    ///    if let Some(fails) = vdb.batch_update(vec![
    ///        (key, updates[0].clone()),
    ///        (key, updates[1].clone()),
    ///        (key1, update1),
    ///        (key2, update2),
    ///        (key3, updates3[0].clone()),
    ///        (key3, updates3[1].clone()),
    ///    ]) {
    ///        panic!("Some updates failed {:?}", fails);
    ///    };
    //////
    /// ```
    pub fn batch_update(
        &mut self,
        mut updates: Vec<(PublicKey, AccountFieldsUpdate)>,
    ) -> Option<Vec<(PublicKey, Vec<AccountFieldsUpdate>, Result<(), VrrbDbError>)>> {
        // Store and return all failures as (PublicKey, AllPushedUpdates, Error)
        // This way caller is provided with all info -> They know which accounts were not modified, have a list of all updates to try again
        // And an error thrown so that they can fix it
        let mut failed =
            Vec::<(PublicKey, Vec<AccountFieldsUpdate>, Result<(), VrrbDbError>)>::new();

        // We sort updates by nonce (that's impl of Ord in AccountField)
        // This way all provided updates are used in order (doesn't matter for different accounts, but important for multiple ops on single PubKey)
        updates.sort_by(|a, b| a.1.cmp(&b.1));

        // We'll segregate the batch of updates by key (since it's possible that in provided Vec there is a chance that not every PublicKey is unique)
        let mut update_batches = HashMap::<PublicKey, Vec<AccountFieldsUpdate>>::new();
        updates.iter().for_each(|update| {
            if let Some(vec_of_updates) = update_batches.get_mut(&update.0) {
                vec_of_updates.push(update.1.clone());
            } else {
                update_batches.insert(update.0, vec![update.1.clone()]);
            }
        });

        // For each PublicKey we try to apply every AccountFieldsUpdate on a copy of current account
        // if event one fails, the whole batch is abandoned with no changes on VrrbDb
        // when that happens, the key, batch of updates and error are pushed into result vec
        // On success we update the account at given index (PublicKey)
        // We don't need to commit the changes, since we never go back to that key in this function,
        // saving a lot of time (we don't need to wait for all readers to finish)
        update_batches.drain().for_each(|(k, v)| {
            let mut fail: (bool, Result<(), VrrbDbError>) = (false, Ok(()));
            let mut final_account = Account::new();
            match self.read_handle().get(k) {
                Some(mut account) => {
                    for update in v.as_slice() {
                        if let Err(e) = account.update(update.clone()) {
                            fail = (true, Err(e));
                            break;
                        }
                    }
                    final_account = account;
                }
                None => fail = (true, Err(VrrbDbError::AccountDoesntExist(k))),
            }

            if fail.0 {
                failed.push((k, v, fail.1));
            } else {
                self.w.update(k, Box::new(final_account));
            };
        });

        if failed.len() != updates.len() {
            self.commit_changes();
        };

        if failed.len() == 0 {
            return None;
        }

        Some(failed)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum VrrbDbError {
    RecordExists,
    InitWithDebit,
    InitWithNonce,
    AccountDoesntExist(PublicKey),
    InvalidUpdateNonce(u32, u32),
    UpdateFailed(AccountField),
}

impl Database for LeftRightDatabase<K, V> {
    type Error;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        todo!()
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<(), Self::Error> {
        todo!()
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn flush(&self) -> Result<(), Self::Error> {
        todo!()
    }

    fn len(&self) -> Result<usize, Self::Error> {
        todo!()
    }

    fn is_empty(&self) -> Result<bool, Self::Error> {
        todo!()
    }
    //
}
