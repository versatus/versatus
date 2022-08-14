use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    hash::Hash,
    time::{SystemTime, UNIX_EPOCH},
};

// use verifiable::verifiable::Verifiable;
use super::error::MempoolError;
use txn::txn::Txn;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct TxnRecord {
    pub txn_id: String,
    pub txn: String,
    pub txn_timestamp: u128,
    pub txn_added_timestamp: u128,
    pub txn_validated_timestamp: u128,
    pub txn_deleted_timestamp: u128,
}

impl TxnRecord {
    pub fn new(txn: &Txn) -> TxnRecord {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        TxnRecord {
            txn_id: txn.txn_id.clone(),
            txn: txn.to_string(),
            txn_timestamp: txn.txn_timestamp,
            txn_added_timestamp: timestamp,
            txn_validated_timestamp: 0,
            txn_deleted_timestamp: 0,
        }
    }
}

pub struct Mempool<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone + Eq + Hash + evmap::ShallowCopy,
{
    pub txn_store_read: evmap::ReadHandle<K, V>,
    pub txn_store_write: evmap::WriteHandle<K, V>,
    pub db_creation: u128,
    pub last_operation: u128,
}

pub type MempoolDB = Mempool<String, Box<TxnRecord>>;

impl MempoolDB {
    #[allow(unused_mut)]
    pub fn new() -> MempoolDB {
        let (txn_buffer_read, mut txn_buffer_write) = evmap::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        MempoolDB {
            txn_store_read: txn_buffer_read,
            txn_store_write: txn_buffer_write,
            db_creation: timestamp,
            last_operation: timestamp,
        }
    }

    /// Adds a new transaction, makes sure it is unique in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::HashMap;
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// 
    /// let txn = Txn {
    ///     txn_id: String::from("1"),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// };
    /// 
    /// match mempooldb.add_txn(&txn) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// ```
    pub fn add_txn(&mut self, txn: &Txn) -> Result<(), MempoolError> {
        if !self.txn_store_write.contains_key(&txn.txn_id) {
            self.txn_store_write
                .insert(txn.txn_id.clone(), Box::new(TxnRecord::new(txn)));
            self.push();
            Ok(())
        } else {
            Err(MempoolError::TransactionExists)
        }
    }

    /// Adds a batch of new transaction, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    ///
    /// # Examples
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// let mut txns = HashSet::<Txn>::new();
    /// 
    /// txns.insert( Txn {
    ///     txn_id: String::from("1"),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// });
    /// 
    /// match mempooldb.add_txn_batch(&txns) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// ```
    pub fn add_txn_batch(&mut self, txn_batch: &HashSet<Txn>) -> Result<(), MempoolError> {
        txn_batch.iter().for_each(|t| {
            if !self.txn_store_write.contains_key(&t.txn_id) {
                self.txn_store_write
                    .insert(t.txn_id.clone(), Box::new(TxnRecord::new(t)));
            }
        });
        self.push();
        Ok(())
    }

    /// Removes a single transaction identified by id, makes sure it exists in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// let mut txns = HashSet::<Txn>::new();
    /// let txn_id = String::from("1");
    /// 
    /// txns.insert( Txn {
    ///     txn_id: txn_id.clone(),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// });
    /// 
    /// match mempooldb.add_txn_batch(&txns) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///  
    /// match mempooldb.remove_txn_by_id(txn_id.clone()) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///       Err(_) => {
    ///  
    ///      }
    /// };
    /// ```
    pub fn remove_txn_by_id(&mut self, txn_id: String) -> Result<(), MempoolError> {
        if self.txn_store_write.contains_key(&txn_id) {
            self.txn_store_write.empty(txn_id);
            self.push();
            Ok(())
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    /// Removes a single transaction identified by itself, makes sure it exists in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// let txn_id = String::from("1");
    /// 
    /// let txn = Txn {
    ///     txn_id: txn_id.clone(),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// };
    /// 
    /// match mempooldb.add_txn(&txn) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// match mempooldb.remove_txn(&txn) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// ```
    pub fn remove_txn(&mut self, txn: &Txn) -> Result<(), MempoolError> {
        if self.txn_store_write.contains_key(&txn.txn_id) {
            self.txn_store_write.empty(txn.txn_id.clone());
            self.push();
            Ok(())
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    /// Removes a batch of transactions, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// let mut txns = HashSet::<Txn>::new();
    /// let txn_id = String::from("1");
    /// 
    /// txns.insert( Txn {
    ///     txn_id: txn_id.clone(),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// });
    /// 
    /// match mempooldb.add_txn_batch(&txns) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///  
    /// match mempooldb.remove_txn_batch(&txns) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///       Err(_) => {
    ///  
    ///      }
    /// };
    /// ```
    pub fn remove_txn_batch(&mut self, txn_batch: &HashSet<Txn>) -> Result<(), MempoolError> {
        txn_batch.iter().for_each(|t| {
            if self.txn_store_write.contains_key(&t.txn_id) {
                self.txn_store_write.empty(t.txn_id.clone());
            }
        });
        self.push();
        Ok(())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// let mut txns = HashSet::<Txn>::new();
    /// let txn_id = String::from("1");
    /// 
    /// txns.insert( Txn {
    ///     txn_id: txn_id.clone(),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// });
    /// 
    /// match mempooldb.add_txn_batch(&txns) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///
    /// if let Some(txn) = mempooldb.get_txn(&txn_id) {
    ///     assert_eq!(1, mempooldb.size());
    /// } else {
    ///     panic!("Transaction missing !");
    /// };
    /// ```
    pub fn get_txn(&mut self, txn_id: &String) -> Option<Txn> {
        if !txn_id.is_empty() {
            self.txn_store_read
                .factory()
                .handle()
                .get_one(txn_id)
                .map(|t| Txn::from_string(&t.txn))
        } else {
            None
        }
    }

    /// Retrieves actual size of the mempooldb.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::MempoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut mempooldb = MempoolDB::new();
    /// let mut txns = HashSet::<Txn>::new();
    /// let txn_id = String::from("1");
    /// 
    /// txns.insert( Txn {
    ///     txn_id: txn_id.clone(),
    ///     txn_timestamp: 0,
    ///     sender_address: String::from("aaa1"),
    ///     sender_public_key: String::from("RSA"),
    ///     receiver_address: String::from("bbb1"),
    ///     txn_token: None,
    ///     txn_amount: 0,
    ///     txn_payload: String::from("x"),
    ///     txn_signature: String::from("x"),
    ///     validators: HashMap::<String, bool>::new(),
    ///     nonce: 0,
    /// });
    /// 
    /// match mempooldb.add_txn_batch(&txns) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///
    /// assert_eq!(1, mempooldb.size());
    /// ```
    pub fn size(&self) -> usize {
        self.txn_store_write.len()
    }

    pub fn validate(&mut self, _txn: &Txn) -> Result<(), MempoolError> {
        Ok(())
    }

    pub fn validate_by_id(&mut self, _txn_id: String) -> Result<(), MempoolError> {
        Ok(())
    }

    fn push(&mut self) {
        self.txn_store_write.refresh();
        self.last_operation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
    }
}
