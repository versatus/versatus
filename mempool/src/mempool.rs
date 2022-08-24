use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    hash::Hash,
    time::{SystemTime, UNIX_EPOCH},
};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;

use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};

use txn::txn::Txn;
use super::error::MempoolError;
use super::txn_validator::TxnValidator;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct TxnRecord {
    pub txn_id: String,
    pub txn: String,
    pub txn_timestamp: u128,
    pub txn_added_timestamp: u128,
    pub txn_validated_timestamp: u128,
    pub txn_rejected_timestamp: u128,
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
            ..Default::default()
        }
    }

    pub fn new_by_id(txn_id: &String) -> TxnRecord {

        TxnRecord {
            txn_id: txn_id.clone(),
            ..Default::default()
        }
    }
}

impl Default for TxnRecord {
    fn default() -> Self {
        TxnRecord {
            txn_id: String::from(""),
            txn: String::from(""),
            txn_timestamp: 0,
            txn_added_timestamp: 0,
            txn_validated_timestamp: 0,
            txn_rejected_timestamp: 0,
            txn_deleted_timestamp: 0
        }
    }
}

pub type MempoolType = IndexMap<String, TxnRecord, FxBuildHasher>;

pub enum TxnStatus {
    Pending,
    Validated,
    Rejected
}

#[derive(Clone, PartialEq, Eq)]
pub struct Mempool {
    pub pending: MempoolType,
    pub validated: MempoolType,
    pub rejected: MempoolType
}

impl Default for Mempool {
    fn default() -> Self {

        // TODO - to be moved to a common configuration file
        let initial_mempool_capacity = 100;

        Mempool { 
            pending: MempoolType::with_capacity_and_hasher(initial_mempool_capacity, <_>::default()),
            validated: MempoolType::with_capacity_and_hasher(initial_mempool_capacity, <_>::default()),
            rejected: MempoolType::with_capacity_and_hasher(initial_mempool_capacity, <_>::default())
        }
    }
}

pub enum MempoolOp {
    Add(TxnRecord, TxnStatus),
    Remove(TxnRecord, TxnStatus),
}

impl Absorb<MempoolOp> for Mempool
{
    fn absorb_first(&mut self, op: &mut MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(recdata, status) => {
                match status {
                    TxnStatus::Pending => {
                        self.pending.insert(recdata.txn_id.clone(), recdata.clone());
                    },
                    TxnStatus::Validated => {
                        self.validated.insert(recdata.txn_id.clone(), recdata.clone());
                    },
                    TxnStatus::Rejected => {
                        self.rejected.insert(recdata.txn_id.clone(), recdata.clone());
                    },
                }
            },
            MempoolOp::Remove(recdata, status) => {
                match status {
                    TxnStatus::Pending => {
                        self.pending.remove(&recdata.txn_id);
                    },
                    TxnStatus::Validated => {
                        self.validated.remove(&recdata.txn_id);
                    },
                    TxnStatus::Rejected => {
                        self.rejected.remove(&recdata.txn_id);
                    },
                }
            },
        }
    }

    fn absorb_second(&mut self, op: MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(recdata, status) => {
                match status {
                    TxnStatus::Pending => {
                        self.pending.insert(recdata.txn_id.clone(), recdata.clone());
                    },
                    TxnStatus::Validated => {
                        self.validated.insert(recdata.txn_id.clone(), recdata.clone());
                    },
                    TxnStatus::Rejected => {
                        self.rejected.insert(recdata.txn_id.clone(), recdata.clone());
                    },
                }
            },
            MempoolOp::Remove(recdata, status) => {
                match status {
                    TxnStatus::Pending => {
                        self.pending.remove(&recdata.txn_id);
                    },
                    TxnStatus::Validated => {
                        self.validated.remove(&recdata.txn_id);
                    },
                    TxnStatus::Rejected => {
                        self.rejected.remove(&recdata.txn_id);
                    },
                }
            },
        }
    }

    fn drop_first(self: Box<Self>) {
    }

    fn drop_second(self: Box<Self>) {
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}

pub struct LeftRightMemPoolDB {
    pub read: ReadHandle<Mempool>,
    pub write: WriteHandle<Mempool, MempoolOp>,
}

impl LeftRightMemPoolDB {

    pub fn new() -> Self {
        let (write, read)
            = left_right::new::<Mempool, MempoolOp>();

        LeftRightMemPoolDB {
            read: read,
            write: write
        }
    }

    pub fn get(&self) -> Option<Mempool> {
        self.read
            .enter()
            .map(|guard| guard.clone())
    }

    pub fn factory(&self) -> ReadHandleFactory<Mempool> {
        self.read.factory()
    }

    /// Adds a new transaction, makes sure it is unique in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::HashMap;
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn(&txn) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// 
    /// assert_eq!(1, lrmempooldb.size());
    /// ```
    pub fn add_txn(&mut self, txn: &Txn) -> Result<(), MempoolError> {

        let op = MempoolOp::Add(TxnRecord::new(txn), TxnStatus::Pending);
        self.write.append(op);
        self.publish();
        Ok(())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn_batch(&txns) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///
    /// if let Some(txn) = lrmempooldb.get_txn(&txn_id) {
    ///     assert_eq!(1, lrmempooldb.size());
    /// } else {
    ///     panic!("Transaction missing !");
    /// };
    /// ```
    pub fn get_txn(&mut self, txn_id: &String) -> Option<Txn> {
        if !txn_id.is_empty() {
            self.get()
                .and_then(|map| {
                    map
                        .pending
                        .get(txn_id)
                        .and_then(|t| Some(Txn::from_string(&t.txn)))
                })
        } else {
            None
        }
    }

    pub fn get_txn_record(&mut self, txn_id: &String) -> Option<TxnRecord> {

        if !txn_id.is_empty() {
            self.get()
                .and_then(|map| {
                    map
                        .validated
                        .get(txn_id)
                        .cloned()
                })
        } else {
            None
        }            
    }

    pub fn get_txn_record_validated(&mut self, txn_id: &String) -> Option<TxnRecord> {
        if !txn_id.is_empty() {
            self.get()
                .and_then(|map| {
                    map
                        .validated
                        .get(txn_id)
                        .cloned()
                })
        } else {
            None
        }
    }

    pub fn get_txn_record_rejected(&mut self, txn_id: &String) -> Option<TxnRecord> {
        if !txn_id.is_empty() {
            self.get()
                .and_then(|map| {
                    map
                        .rejected
                        .get(txn_id)
                        .cloned()
                })
        } else {
            None
        }
    }

    /// Adds a batch of new transaction, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    ///
    /// # Examples
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn_batch(&txns) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// 
    /// assert_eq!(1, lrmempooldb.size());
    /// ```
    pub fn add_txn_batch(&mut self, txn_batch: &HashSet<Txn>) -> Result<(), MempoolError> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Add(TxnRecord::new(t), TxnStatus::Pending));
        });
        self.publish();
        Ok(())
    }

    /// Removes a single transaction identified by id, makes sure it exists in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn_batch(&txns) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///  
    /// match lrmempooldb.remove_txn_by_id(txn_id.clone()) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///       Err(_) => {
    ///  
    ///      }
    /// };
    /// 
    /// assert_eq!(0, lrmempooldb.size());
    /// ```
    pub fn remove_txn_by_id(&mut self, txn_id: String) -> Result<(), MempoolError> {
        self.write.append(MempoolOp::Remove(TxnRecord::new_by_id(&txn_id), TxnStatus::Pending));
        self.publish();
        Ok(())
    }

    /// Removes a single transaction identified by itself, makes sure it exists in db.
    /// Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn(&txn) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// match lrmempooldb.remove_txn(&txn) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    /// 
    /// assert_eq!(0, lrmempooldb.size());
    /// ```
    pub fn remove_txn(&mut self, txn: &Txn) -> Result<(), MempoolError> {
        self.write.append(MempoolOp::Remove(TxnRecord::new(txn), TxnStatus::Pending));
        self.publish();
        Ok(())
    }

    /// Removes a batch of transactions, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn_batch(&txns) {
    ///      Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///  
    /// match lrmempooldb.remove_txn_batch(&txns) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///       Err(_) => {
    ///  
    ///      }
    /// };
    /// 
    /// assert_eq!(0, lrmempooldb.size());
    /// ```
    pub fn remove_txn_batch(&mut self, txn_batch: &HashSet<Txn>) -> Result<(), MempoolError> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Remove(TxnRecord::new(t), TxnStatus::Pending));
        });
        self.publish();
        Ok(())
    }

    pub fn validate_all(&mut self) -> Result<(), MempoolError> {

        self.get()
            .and_then(|map| {
                map
                    .pending
                    .iter().for_each(|rec_entry| {
                        let txn = Txn::from_string(&rec_entry.1.txn);
                        match self.validate(&txn) {
                            Ok(_) => {
                                self.write.append(MempoolOp::Remove(rec_entry.1.clone(), TxnStatus::Pending));
                                self.write.append(MempoolOp::Add(rec_entry.1.clone(), TxnStatus::Validated));
                            },
                            Err(_) => {
                                self.write.append(MempoolOp::Remove(rec_entry.1.clone(), TxnStatus::Pending));
                                self.write.append(MempoolOp::Add(rec_entry.1.clone(), TxnStatus::Rejected));
                            }
                        }
                    });
                Some(map)
            });
        self.publish();
        Ok(())
    }

    pub fn validate(&mut self, txn: &Txn) -> Result<(), MempoolError> {
        
        TxnValidator::new(txn).validate().map_err(|_| MempoolError::TransactionInvalid)
    }

    pub fn is_txn_validated(&mut self, txn: &Txn) -> Result<u128, MempoolError> {
        if let Some(txn_record_validated) = self.get_txn_record_validated(&txn.txn_id) {
            Ok(txn_record_validated.txn_validated_timestamp)
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    pub fn is_txn_rejected(&mut self, txn: &Txn) -> Result<u128, MempoolError> {
        if let Some(txn_record_rejected) = self.get_txn_record_rejected(&txn.txn_id) {
            Ok(txn_record_rejected.txn_validated_timestamp)
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    /// Retrieves actual size of the mempooldb.
    ///
    /// # Examples
    ///
    /// ```
    /// use mempool::mempool::LeftRightMemPoolDB;
    /// use txn::txn::Txn;
    /// use std::collections::{HashSet, HashMap};
    /// 
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
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
    /// match lrmempooldb.add_txn_batch(&txns) {
    ///     Ok(_) => {
    ///         
    ///     },
    ///     Err(_) => {
    /// 
    ///     }
    /// };
    ///
    /// assert_eq!(1, lrmempooldb.size());
    /// ```
    pub fn size(&self) -> usize {
        if let Some(map) = self.get() {
            map.pending.len()
        } else {
            0
        }
    }

    fn publish(&mut self) {
        self.write.publish();
    }

}
