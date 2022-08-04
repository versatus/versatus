use std::{
    hash::{Hash},
    time::SystemTime,
    fmt::{self}, collections::HashMap, cmp::Ordering,
};

use sha2::{Sha256, Digest};
use secp256k1::{
    PublicKey,
};
use serde::{Serialize, Deserialize};


/// Stores information about given account. 
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Account {
    pub hash: String,
    pub nonce: u32,
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
    /// use lrdb::Account;
    /// 
    /// let account = Account::new();
    /// ```
    pub fn new() -> Account{
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

        Account{
           hash,
           nonce,
           credits,
           debits,
           storage,
           code
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
    fn update_single_field_no_hash(&mut self, value: AccountField) -> Result<(), VrrbDBError> {
        match value {
            AccountField::Credits(credits) => {
                match self.credits.checked_add(credits) {
                    Some(new_amount) => {
                        self.credits = new_amount
                    },
                    None => {
                        return Err(VrrbDBError::UpdateFailed(value))
                    }
                }
            },
            AccountField::Debits(debits) => {
                match self.debits.checked_add(debits) {
                    Some(new_amount) => {
                        self.debits = new_amount
                    },
                    None => {
                        return Err(VrrbDBError::UpdateFailed(value))
                    }
                }
            },

            // Should the storage be impossible to delete?
            AccountField::Storage(storage) => {
                self.storage = storage;
            },

            // Should the code be impossible to delete?
            AccountField::Code(code) => {
                self.code = code;
            }, 
            other_field => { return Err(VrrbDBError::UpdateFailed(other_field)) }
        } 
        Ok(())
    } 

    /// Updates single field of the struct. Then updates it's nonce. 
    /// Finaly recalculates and updates the hash. Might return an error.
    /// 
    /// # Arguments:
    /// * `update` - An AccountField enum specifying which field update (with what value) 
    /// 
    /// # Examples: 
    /// ```
    /// use lrdb::{Account, AccountField};
    /// let mut account = Account::new();
    /// account.update_field(AccountField::Credits(300));
    /// 
    /// assert_eq!(account.credits, 300);
    /// ```
    pub fn update_field(&mut self, update: AccountField) -> Result<(), VrrbDBError> {
        let res = self.update_single_field_no_hash(update);
        self.bump_nonce();
        self.update_hash();
        res
    }


 
    // TODO: Verify if

    /// Updates all fields of the account struct accoring to supplied AccountFieldsUpdate struct.
    /// Requires provided nonce (update's nonce) to be exactly one number higher than accounts nonce.
    /// Recalculates hash. Might return an error.
    /// 
    /// # Arguments:
    /// * `update` - An AccountFieldsUpdate struct containing instructions to update each field of the account struct.
    /// 
    /// # Example: 
    /// ```
    /// use lrdb::{Account, AccountFieldsUpdate}; 
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
    pub fn update(&mut self, update: AccountFieldsUpdate) -> Result<(), VrrbDBError>{
        if self.nonce+1 != update.nonce {
            return Err(VrrbDBError::InvalidUpdateNonce(self.nonce, update.nonce))
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

    fn bump_nonce(&mut self) {
        self.nonce += 1;
    }
}

/// Enum containing options for updates - used to update value of single field in account struct.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AccountField {
    Credits(u64),
    Debits(u64),
    Storage(Option<String>),
    Code(Option<String>)
}


// VrrbDB struct containing evmap implementation of left-right contained hashmap
/// Struct representing the LeftRight Database. 
/// ReadHandleFactory provides a way of creating new ReadHandles to the database.
/// WriteHandles provides a way to gain write access to the database.
/// Last denotes the lastest `refresh` of the database.
#[allow(dead_code)]
pub struct LeftRightDatabase<K, V> 
where
    K: Clone + Eq + Hash,
    V: Clone + Eq + evmap::ShallowCopy,
{
    r: evmap::ReadHandleFactory<K,V, ()>,
	w: evmap::WriteHandle<K, V, ()>,
	last: std::time::SystemTime,
}

/// NOTE: Boxing the Account so it's `ShallowCopy`
/// ShallowCopy is unsafe. To work properly it requires that type is never modified.
#[allow(dead_code)]
/// Type wrapping LeftRightDatabase with non-generic arguments used in this implementation.
type VrrbDB = LeftRightDatabase<PublicKey, Box::<Account>>;


/// Struct representing the desired updates to be applied to account.
#[derive(Clone, PartialEq, Eq)]
pub struct AccountFieldsUpdate {
    pub nonce: u32,
    pub credits: Option<u64>,
    pub debits: Option<u64>,
    pub storage: Option<Option<String>>,
    pub code: Option<Option<String>>,
}

// The AccountFieldsUpdate will be compared by `nonce`. This way the updates can be properly scheduled.
impl Ord for AccountFieldsUpdate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nonce.cmp(&other.nonce)
    }
}

impl PartialOrd for AccountFieldsUpdate{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


/// TODO: docs
impl VrrbDB 
{ 
    /// Returns new, empty account.
    /// 
    /// Examples: 
    /// ```
    /// use lrdb::VrrDB;
    /// 
    /// let vdb = VrrDB::new();
    /// ```
    pub fn new() -> Self {
        let (vrrbdb_reader, mut vrrbdb_writer) = evmap::new();
        // This is required to set up oplog
        // Otherwise there's no way to keep track of already inserted keys (before refresh)
        vrrbdb_writer.refresh();
        Self{
            r: vrrbdb_reader.factory(),
            w: vrrbdb_writer, 
            last: SystemTime::now()
        }
    }

    // Commits uncommited changes 
    /// Commits the pending changes to the underlying evmap by calling `refresh()`
    /// Will wait for EACH ReadHandle to be consumed. 
    fn commit_changes(&mut self) {
        self.w.refresh();
        self.last = SystemTime::now();
    }


    // TODO: Maybe initialize is better name for that?
    fn insert_uncommited(&mut self, pubkey: PublicKey, account: Account) -> Result<(), VrrbDBError>{

        // Cannot insert new account with debit
        if account.debits != 0 {
            return Err(VrrbDBError::InitWithDebit)
        }

        // Cannot insert account with nonce bigger than 0 
        if account.nonce != 0 {
            return Err(VrrbDBError::InitWithNonce)
        }

        // Check oplog for uncommited inserts for given key
        // That should only be possible if the same PublicKey is used in batch_insert
        let mut found = false;
        self.w.pending().iter().for_each(|op| {
            if let evmap::Operation::Add::<PublicKey, Box::<Account>>(key, _) = op {
               if pubkey == *key {
                    found = true;    
                } 
            }
        });
 

        match self.w.contains_key(&pubkey) || found {
            true => return Err(VrrbDBError::RecordExists),
            false => {  
                self.w.insert(pubkey, Box::new(account));
                return Ok(())
            } 
        }
    }

    /// Inserts new account into VrrDB. 
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
    /// use lrdb::{Account, VrrDB, VrrDBError};
    /// use secp256k1::PublicKey;
    /// 
    /// let account = Account::new();
    /// let mut vdb = VrrDB::new();
    /// let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    /// let added = vdb.insert(key, account);
    /// assert_eq!(added, Ok(()));
    /// ```
    /// 
    /// Failed inserts: 
    /// 
    /// ```
    /// use lrdb::{Account, VrrDB};
    /// use secp256k1::PublicKey;
    /// 
    /// let account = Account::new();
    /// 
    /// // That will fail, since nonce should be 0 
    /// account.nonce = 10;
    /// let mut vdb = VrrDB::new();
    /// let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    /// let added = vdb.insert(key, account);
    /// 
    /// assert_eq!(added, Err(VrrDBError::InitWithNonce));
    ///  
    /// account.nonce = 0;
    /// account.debit = 10;
    /// 
    /// // This will fail since the debit should be 0
    /// added = vdb.insert(key,account);
    /// 
    /// assert_eq!(added, Err(VrrDBError::InitWithDebit));
    /// ``` 
    pub fn insert(&mut self, pubkey: PublicKey, account: Account) -> Result<(), VrrbDBError>{
        self.insert_uncommited(pubkey, account)?;
        self.commit_changes();
        Ok(())
    }

    // Iterates over provided (PublicKey,DBRecord) pairs, inserting valid ones into the db
    // Returns Option with vec of NOT inserted (PublicKey,DBRecord,e) pairs
    // e being the error which prevented (PublicKey,DBRecord) from being inserted
    fn batch_insert_uncommited(&mut self, inserts: Vec<(PublicKey,Account)>) -> Option<Vec<(PublicKey,Account, VrrbDBError)>> {
        let mut failed_inserts: Vec<(PublicKey,Account,VrrbDBError)> = vec![];

        inserts.iter().for_each( |item| {
            let (k,v) = item;
            if let Err(e) = self.insert_uncommited(k.clone(), v.clone()) {
                failed_inserts.push((k.clone(), v.clone(),e));
            } 
        });
        
        if failed_inserts.is_empty() { return None }
        else {return Some(failed_inserts)}
    }

    /// Inserts a batch of accounts provided in a vector
    /// 
    /// Arguments:
    /// 
    /// * `inserts` - Vec<(PublicKey, Account)> - Vector of tuples - containing target (unique) PublicKey and Account struct to be inserted
    /// 
    /// Examples:
    /// ``` 
    /// use lrdb::{Account, VrrDB};
    /// use secp256k1::PublicKey;
    /// 
    /// let mut vrrbdb = VrrbDB::new();
    ///    
    /// let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    /// let mut account1 = Account::new();
    /// account1.update_field(AccountField::Credits(100));
    ///  
    /// let mut account2 = Account::new();
    /// account2.update_field(AccountField::Credits(237));
    ///    
    /// let mut account3 = Account::new();
    /// account3.update_field(AccountField::Credits(500));
    ///
    /// vrrbdb.batch_insert(vec![(keys[0], account1), (keys[1], account2), (keys[2], account3)]);
    ///
    /// if let Some(account) = vrrbdb.get(keys[1]) {
    /// assert_eq!(account.credits, 237); 
    /// ```
    pub fn batch_insert(&mut self, inserts: Vec<(PublicKey,Account)>) -> Option<Vec<(PublicKey,Account, VrrbDBError)>> {
       let failed_inserts = self.batch_insert_uncommited(inserts);
       self.commit_changes();
       failed_inserts 
    }

    /// Retain returns new VrrDB with witch all Accounts that fulfill `filter` cloned to it.
    /// 
    /// Arguments: 
    /// 
    /// * `filter` - closure specifying filter for cloned accounts
    /// 
    /// Examples: 
    ///    ```
    ///    use lrdb::{VrrbDB, Account, AccountField};
    /// 
    ///    let mut vdb = VrrbDB::new();
    ///    
    ///    let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
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
    ///    vdb.batch_insert(vec![(keys[0], account), (keys[1], account1), (keys[2], account2.clone()), (keys[3], account3)]);
    ///
    ///    let filtered = vdb.retain(|acc| {acc.credits >= 300 && acc.credits < 500});
    ///     
    ///    assert_eq!(filtered.len(), 1);
    ///
    /// ```
    pub fn retain<F>(&self, mut filter: F) -> VrrbDB 
    where 
        F: FnMut(&Account) -> bool 
    {
        let mut subdb = VrrbDB::new();
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
    ///  use lrdb::{VrrbDB, Account, AccountField};
    /// 
    ///  let mut vdb = VrrbDB::new();
    ///    
    ///  let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    ///  let mut account = Account::new(); 
    ///  account.update_field(AccountField::Credits(123));
    /// 
    ///  vdb.insert(key, record);
    /// 
    ///  if let Some(account) = vdb.get(key) {
    ///     assert_eq!(account.credits, 123);
    ///  }
    /// ```
    pub fn get(&self, key: PublicKey) -> Option<Account> {
        self.r.handle().get_and(&key, |x| return *x[0].clone())
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
    ///  use lrdb::{VrrbDB, Account, AccountField};
    /// 
    ///  let mut vdb = VrrbDB::new();
    ///    
    ///  let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    ///  let key2 = PublicKey::from_str("d5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    ///  
    ///  let mut account = Account::new(); 
    ///  account.update_field(AccountField::Credits(123));
    /// 
    ///  let mut account2 = Account::new();
    ///  account.update_field(AcccountField::Credits(257));
    ///   
    ///  vdb.batch_insert(vec![(key, account), (key2,account2)]);
    /// 
    ///  let result = vdb.batch_get(vec![key, key2]);
    ///  
    ///  if let Some(acc) = result[1] {
    ///     assert_eq!(acc.credits, 257);
    ///  }
    ///  
    /// ```
    pub fn batch_get(&self, keys: Vec<PublicKey>) -> HashMap<PublicKey, Option<Account>> {
        let mut accounts = HashMap::<PublicKey, Option<Account>>::new(); 
        keys.iter().for_each(|k| {
            accounts.insert(k.clone(), self.get(*k));
        }); 
        accounts
    }

    // If possible, updates the account. Otherwise returns an error
    fn update_uncommited(&mut self, key: PublicKey, update: AccountFieldsUpdate) -> Result<(), VrrbDBError>{ 
        match self.get(key) {
            Some(mut account) => {
                account.update(update)?;
                self.w.update(key, Box::new(account));

                return Ok(())
            }, 
            None => {
                return Err(VrrbDBError::AccountDoesntExist(key))
            }
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
    ///    use lrdb::{VrrbDB, Account, AccountField, AccountFieldsUpdate};
    /// 
    ///    let mut vdb = VrrbDB::new();
    ///    
    ///    let key = PublicKey::from_str("c5637acae723ed0cd810a28862e8f90562b216794bd72c95ed7807cfb650a48e8b479397fd94ce0f1ca90640820dff3b717b92b7a57477ca2a3d9fec409ac88d".to_string());
    ///    let mut account = Account::new(); 
    ///    account.update_field(AccountField::Credits(123));
    ///     
    ///    let update = AccountFields {
    ///        nonce: 1, 
    ///        credits: Some(100),
    ///        debits: Some(50),
    ///        storage: None,
    ///        code: None, 
    ///    }
    ///    
    ///    vdb.update(key, update);
    /// 
    ///    if let Some(acc) = vdb.get(key) {
    ///       assert_eq!(acc.credits,100);
    ///       assert_eq!(acc.debits, 50);    
    ///    }
    ///   
    /// ```
    pub fn update(&mut self, key: PublicKey, update: AccountFieldsUpdate) -> Result<(), VrrbDBError>{ 
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
    /// 
    /// 
    /// ```
    pub fn batch_update(&mut self, mut updates: Vec<(PublicKey, AccountFieldsUpdate)>) -> Vec<(PublicKey, Vec<AccountFieldsUpdate>, Result<(),VrrbDBError>)>{
        // Store and return all failures as (PublicKey, AllPushedUpdates, Error)
        // This way caller is provided with all info -> They know which accounts were not modified, have a list of all updates to try again
        // And an error thrown so that they can fix it
        let mut result = Vec::<(PublicKey, Vec<AccountFieldsUpdate>, Result<(),VrrbDBError>)>::new();

        
        // We sort updates by nonce (that's impl of Ord in AccountField)
        // This way all provided updates are used in order (doesn't matter for different accounts, but important for multiple ops on single PubKey)
        updates.sort_by(|a,b| a.1.cmp(&b.1));

        
        // We'll segregate the batch of updates by key (since it's possible that in provided Vec there is a chance that not every PublicKey is unique)
        let mut update_batches = HashMap::<PublicKey, Vec<AccountFieldsUpdate>>::new();
        updates.iter().for_each(|update| {
            if let Some(vec_of_updates) =  update_batches.get_mut(&update.0) {
                vec_of_updates.push(update.1.clone());
            } else {
                update_batches.insert(update.0, vec![update.1.clone()]);
            }
        });
       
        // For each PublicKey we try to apply every AccountFieldsUpdate on a copy of current account
        // if event one fails, the whole batch is abandoned with no changes on VrrbDB
        // when that happens, the key, batch of updates and error are pushed into result vec
        // On success we update the account at given index (PublicKey)
        // We don't need to commit the changes, since we never go back to that key in this function, 
        // saving a lot of time (we don't need to wait for all readers to finish)
        update_batches.drain().for_each(|(k,v)| {
            let mut fail: (bool, Result<(), VrrbDBError>) = (false, Ok(()));
            let mut final_account = Account::new();
            match self.get(k) {
                Some(mut account) => {
                    for update in v.as_slice() {
                        if let Err(e) = account.update(update.clone()) {
                            fail = (true, Err(e));
                            break;
                        }
                    }
                    final_account = account;
                },
                None => {
                    fail = (true, Err(VrrbDBError::AccountDoesntExist(k)))
                }
            } 
            
            if fail.0 { result.push((k,v,fail.1));}
            else { self.w.update(k, Box::new(final_account));};
        });

        if result.len() != updates.len() { self.commit_changes(); };
        result
    }
    
}

#[derive(PartialEq, Eq, Debug)]
pub enum VrrbDBError {
    RecordExists,
    InitWithDebit,
    InitWithNonce,
    AccountDoesntExist(PublicKey),
    InvalidUpdateNonce(u32, u32),
    UpdateFailed(AccountField)
}

impl fmt::Display for AccountField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.clone() {
            AccountField::Code(code) => {
                match code {
                    None => {
                        write!(f ,"Code: None.")
                    },
                    Some(code) => {
                        write!(f, "Code: {}.", code)
                    }
                }
            },
            AccountField::Credits(credits) => {
                write!(f, "Credits: {}.", credits)
            },
            AccountField::Debits(debits) => {
                write!(f, "Debits: {}.", debits)
            },  
            AccountField::Storage(storage) => {
                match storage {
                    None => {
                        write!(f, "Storage: None.")
                    },
                    Some(storage) => {
                        write!(f, "Storage: {}.", storage)
                    }
                }
            }

        }
    }
}

impl fmt::Display for VrrbDBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<VrrbDBError> for String{
    fn from(error: VrrbDBError) -> String{
        match error {
            VrrbDBError::RecordExists => String::from("Failed to insert the item. Record already exists"),
            VrrbDBError::InitWithDebit => String::from("Failed to insert the account. Initial debit should be 0."),
            VrrbDBError::InitWithNonce => String::from("Failed to insert the account. Nonce should be 0."),
            VrrbDBError::AccountDoesntExist(key) => String::from(format!("The provided account :0x{:x} does not exist.", key)),
            VrrbDBError::InvalidUpdateNonce(acc_nonce, update_nonce) => String::from(format!("The provided update with nonce: {} seems to be outdated with current accounts nonce: {}", update_nonce, acc_nonce)),
            VrrbDBError::UpdateFailed(field) => String::from(format!("Failed to update:{{{}}}", field)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    use secp256k1::generate_keypair;
    
    fn new_random_keys() -> Vec<PublicKey> {
    
        let mut rng = StdRng::from_entropy();
        let mut res: Vec<PublicKey> = vec![];
        for _ in 0..10 {
            let (_, pubkey) = generate_keypair(&mut rng);
            res.push(pubkey);
        }
        res
     }
    

    #[test]
    fn creates_new_database() {
        let _vrrb_db = VrrbDB::new(); 
    }

    #[test]
    fn insert_single_account_with_commitment(){
        let keys = new_random_keys();

        let mut record = Account::new();
        record.update_field(AccountField::Credits(100));

        let mut vrrbdb = VrrbDB::new();
        if let Err(_) = vrrbdb.insert(keys[0], record){
            panic!("Failed to insert account with commitment")
        }

        vrrbdb.r.handle().get_and(&keys[0], |record| {
            let acc = *record[0].clone();
            assert_eq!(acc.credits, 100);
        });

    }

    #[test]
    fn fail_to_insert_with_nonce() {
        let keys = new_random_keys();
        let mut vrrbdb = VrrbDB::new();
        let mut record = Account::new();

        record.bump_nonce();

        let result = vrrbdb.insert(keys[0], record);
        assert_eq!(result, Err(VrrbDBError::InitWithNonce))
    }
 
    #[test]
    fn fail_to_insert_with_debit() {
        let keys = new_random_keys();
        let mut vrrbdb = VrrbDB::new();
        let mut record = Account::new();

        record.update_field(AccountField::Debits(100));

        let result = vrrbdb.insert(keys[0], record);
        assert_eq!(result, Err(VrrbDBError::InitWithDebit))
    }
    
    #[test]
    fn inserts_multiple_valid_k_v_pairs_with_commitment(){
        let keys = new_random_keys();
        let mut vrrbdb = VrrbDB::new();
        
        let mut record1 = Account::new();
        record1.update_field(AccountField::Credits(100));
        record1.update_field(AccountField::Debits(0));
        
        let mut record2 = Account::new();
        record2.update_field(AccountField::Credits(237));
        record2.update_field(AccountField::Debits(0));
        
        let mut record3 = Account::new();
        record3.update_field(AccountField::Credits(500));
        record3.update_field(AccountField::Debits(0));
        
        vrrbdb.batch_insert(vec![(keys[0], record1), (keys[1], record2), (keys[2], record3)]);

        if let Some(account) = vrrbdb.get(keys[1]) {
            assert_eq!(account.credits, 237); 
        }
    } 


    #[test]
    fn insert_for_same_key_multiple_times_without_commitements_should_be_impossible(){
        let keys = new_random_keys();
        let mut vrrbdb = VrrbDB::new();
        
        let mut record1 = Account::new();
        record1.update_field(AccountField::Credits(100));
        record1.update_field(AccountField::Debits(0));
        
        let mut record2 = Account::new();
        record2.update_field(AccountField::Credits(237));
        record2.update_field(AccountField::Debits(0));


        if let Err(e) = vrrbdb.insert_uncommited(keys[0], record1) {
            panic!("{}",e)
        }


        match vrrbdb.insert_uncommited(keys[0], record2) {
            Err(e) => {
                assert_eq!(e, VrrbDBError::RecordExists)
            },
            Ok(_) => {
                panic!("Multiple inserts for the same key!");
            }
        }
    }

    #[test]
    fn inserts_multiple_k_v_pairs_some_invalid_with_commitment(){
        let keys = new_random_keys();
        let mut vrrbdb = VrrbDB::new();
        
        let mut record1 = Account::new();
        record1.update_field(AccountField::Credits(100));
        record1.update_field(AccountField::Debits(0));
        
        let mut record2 = Account::new();
        record2.update_field(AccountField::Credits(237));
        record2.update_field(AccountField::Debits(0));
        
        let mut record3 = Account::new();
        record3.update_field(AccountField::Credits(500));
        record3.update_field(AccountField::Debits(500));
        
        
        
        match vrrbdb.batch_insert(vec![(keys[0], record1.clone()), (keys[0], record2.clone()), (keys[2], record3.clone())]) {
            None => { panic!("Should fail.")},
            Some(fails) => {
                let expected = vec![(keys[0], record2, VrrbDBError::RecordExists), (keys[2], record3, VrrbDBError::InitWithDebit)];
                for i in 0..2 {
                    assert_eq!(expected[i].0, fails[i].0);
                    assert_eq!(expected[i].1, fails[i].1);
                    assert_eq!(expected[i].2, fails[i].2);
                }
            }
        } 
    }

    // TODO: More advanced filters once VrrbDB has update functionality
    #[test]
    fn retain_properly_filters_the_values(){
        let keys = new_random_keys();
        let mut vdb = VrrbDB::new();
        
        let mut record = Account::new(); 
        record.update_field(AccountField::Credits(123));
        
        let mut record1 = Account::new(); 
        record1.update_field(AccountField::Credits(250));
        
        let mut record2 = Account::new(); 
        record2.update_field(AccountField::Credits(300));

        let mut record3 = Account::new(); 
        record3.update_field(AccountField::Credits(500));

        vdb.batch_insert(vec![(keys[0], record), (keys[1], record1), (keys[2], record2.clone()), (keys[3], record3)]);

        let filtered = vdb.retain(|acc| {acc.credits >= 300 && acc.credits < 500});
        filtered.r.handle().for_each(|key, value| {
            let account = *value[0].clone();
            assert_eq!(*key, keys[2]);
            assert_eq!(account, record2);
            println!("k: {}, v:{:?}", *key, account);
        });
    }

    #[test]
    fn retain_with_some_more_filters() {}

    #[test]
    fn get_should_return_account() {}

    #[test]
    fn get_should_return_none_for_nonexistant_account() {}

    #[test]
    fn update_with_valid_fields_should_work() {}

    #[test]
    fn update_invalid_data_should_return_error() {}
}
