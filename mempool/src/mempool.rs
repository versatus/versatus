use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::{
    collections::HashSet,
    hash::Hash,
    time::{SystemTime, UNIX_EPOCH},
};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;

use std::result::Result as StdResult;

use state::state::NetworkState;
use txn::txn::Txn;

use super::error::MempoolError;

pub type Result<T> = StdResult<T, MempoolError>;

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
            .expect("The system time seems to be set to be earlier than 1970-01-01 00:00:00 UTC")
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
            txn_deleted_timestamp: 0,
        }
    }
}

pub type MempoolType = IndexMap<String, TxnRecord, FxBuildHasher>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TxnStatus {
    Pending,
    Validated,
    Rejected,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Mempool {
    pub pending: MempoolType,
    pub validated: MempoolType,
    pub rejected: MempoolType,
}

impl Default for Mempool {
    fn default() -> Self {
        // TODO - to be moved to a common configuration file
        let initial_mempool_capacity = 100;

        Mempool {
            pending: MempoolType::with_capacity_and_hasher(
                initial_mempool_capacity,
                <_>::default(),
            ),
            validated: MempoolType::with_capacity_and_hasher(
                initial_mempool_capacity,
                <_>::default(),
            ),
            rejected: MempoolType::with_capacity_and_hasher(
                initial_mempool_capacity,
                <_>::default(),
            ),
        }
    }
}

pub enum MempoolOp {
    Add(TxnRecord, TxnStatus),
    Remove(TxnRecord, TxnStatus),
}

impl Absorb<MempoolOp> for Mempool {
    fn absorb_first(&mut self, op: &mut MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.insert(recdata.txn_id.clone(), recdata.clone());
                }
                TxnStatus::Validated => {
                    self.validated
                        .insert(recdata.txn_id.clone(), recdata.clone());
                }
                TxnStatus::Rejected => {
                    self.rejected
                        .insert(recdata.txn_id.clone(), recdata.clone());
                }
            },
            MempoolOp::Remove(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.remove(&recdata.txn_id);
                }
                TxnStatus::Validated => {
                    self.validated.remove(&recdata.txn_id);
                }
                TxnStatus::Rejected => {
                    self.rejected.remove(&recdata.txn_id);
                }
            },
        }
    }

    fn absorb_second(&mut self, op: MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.insert(recdata.txn_id.clone(), recdata.clone());
                }
                TxnStatus::Validated => {
                    self.validated
                        .insert(recdata.txn_id.clone(), recdata.clone());
                }
                TxnStatus::Rejected => {
                    self.rejected
                        .insert(recdata.txn_id.clone(), recdata.clone());
                }
            },
            MempoolOp::Remove(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.remove(&recdata.txn_id);
                }
                TxnStatus::Validated => {
                    self.validated.remove(&recdata.txn_id);
                }
                TxnStatus::Rejected => {
                    self.rejected.remove(&recdata.txn_id);
                }
            },
        }
    }

    fn drop_first(self: Box<Self>) {}

    fn drop_second(self: Box<Self>) {}

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}

// pub trait FetchFiltered {
//     fn fetch_filtered<F>(&self, f: F) -> HashMap<String, TxnRecord>
//     where
//         F: FnMut(&String, &mut TxnRecord) -> bool;

//     fn fetch_filtered_wihtout_repetition<F>(
//         &self,
//         f: F,
//         seen: &mut HashSet<Txn>,
//     ) -> HashMap<String, TxnRecord>
//     where
//         F: FnMut(&String, &mut TxnRecord) -> bool;
// }

pub trait FetchFiltered {
    fn fetch_pending<F>(&self, amount: u32, f: F) -> Vec<TxnRecord>
    where
        F: FnMut(&String, &mut TxnRecord) -> bool;
}

impl FetchFiltered for ReadHandle<Mempool> {
    fn fetch_pending<F>(&self, amount: u32, f: F) -> Vec<TxnRecord>
    where
        F: FnMut(&String, &mut TxnRecord) -> bool,
    {
        if let Some(map) = self.enter().map(|guard| guard.clone()) {
            let mut result = map.pending.clone();
            result.retain(f);
            let mut returned = Vec::<TxnRecord>::new();
            for (_, v) in &result {
                returned.push(v.clone());
            }
            // TODO:  Error - length
            return returned; //[0..amount as usize].to_vec()
        };
        return Vec::<TxnRecord>::new();
    }
}

pub struct LeftRightMemPoolDB {
    pub read: ReadHandle<Mempool>,
    pub write: WriteHandle<Mempool, MempoolOp>,
}

impl LeftRightMemPoolDB {
    /// Creates new Mempool DB
    pub fn new() -> Self {
        let (write, read) = left_right::new::<Mempool, MempoolOp>();

        LeftRightMemPoolDB {
            read: read,
            write: write,
        }
    }

    /// Getter for Mempool DB
    pub fn get(&self) -> Option<Mempool> {
        self.read.enter().map(|guard| guard.clone())
    }

    /// Returns a new ReadHandleFactory, to simplify multithread access.
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
    /// assert_eq!(1, lrmempooldb.size().0);
    /// ```
    pub fn add_txn(&mut self, txn: &Txn) -> Result<()> {
        self.write
            .append(MempoolOp::Add(TxnRecord::new(txn), TxnStatus::Pending))
            .publish();
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
    ///     assert_eq!(1, lrmempooldb.size().0);
    /// } else {
    ///     panic!("Transaction missing !");
    /// };
    /// ```
    pub fn get_txn(&mut self, txn_id: &String) -> Option<Txn> {
        if txn_id.is_empty() {
            return None;
        }

        self.get().and_then(|map| {
            map.pending
                .get(txn_id)
                .and_then(|t| Some(Txn::from_string(&t.txn)))
        })
    }

    /// Getter for an entire pending Txn record
    pub fn get_txn_record(&mut self, txn_id: &String) -> Option<TxnRecord> {
        if txn_id.is_empty() {
            return None;
        }

        self.get().and_then(|map| map.pending.get(txn_id).cloned())
    }

    /// Getter for an entire validated Txn record
    pub fn get_txn_record_validated(&mut self, txn_id: &String) -> Option<TxnRecord> {
        if txn_id.is_empty() {
            return None;
        }

        self.get()
            .and_then(|map| map.validated.get(txn_id).cloned())
    }

    /// Getter for an entire rejected Txn record
    pub fn get_txn_record_rejected(&mut self, txn_id: &String) -> Option<TxnRecord> {
        if txn_id.is_empty() {
            return None;
        }

        self.get().and_then(|map| map.rejected.get(txn_id).cloned())
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
    /// assert_eq!(1, lrmempooldb.size().0);
    /// ```
    pub fn add_txn_batch(
        &mut self,
        txn_batch: &HashSet<Txn>,
        txns_status: TxnStatus,
    ) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write
                .append(MempoolOp::Add(TxnRecord::new(t), txns_status.clone()));
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
    /// assert_eq!(0, lrmempooldb.size().0);
    /// ```
    pub fn remove_txn_by_id(&mut self, txn_id: String) -> Result<()> {
        self.write
            .append(MempoolOp::Remove(
                TxnRecord::new_by_id(&txn_id),
                TxnStatus::Pending,
            ))
            .publish();
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
    /// assert_eq!(0, lrmempooldb.size().0);
    /// ```
    pub fn remove_txn(&mut self, txn: &Txn) -> Result<()> {
        self.write
            .append(MempoolOp::Remove(TxnRecord::new(txn), TxnStatus::Pending))
            .publish();
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
    /// assert_eq!(0, lrmempooldb.size().0);
    /// ```
    // TODO: fix docs
    pub fn remove_txn_batch(
        &mut self,
        txn_batch: &HashSet<Txn>,
        txns_status: TxnStatus,
    ) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write
                .append(MempoolOp::Remove(TxnRecord::new(t), txns_status.clone()));
        });
        self.publish();
        Ok(())
    }

    /// Apply Txn on debits and credits of currect state
    // TODO: to be clarified against the new state representation.
    pub fn apply_txn_on_state(&mut self, _txn: &Txn, _state: &NetworkState) -> Result<()> {
        Ok(())
    }

    /// Was the Txn validated ? And when ?
    pub fn is_txn_validated(&mut self, txn: &Txn) -> Result<u128> {
        if let Some(txn_record_validated) = self.get_txn_record_validated(&txn.txn_id) {
            Ok(txn_record_validated.txn_validated_timestamp)
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    /// Was the Txn rejected ? And when ?
    pub fn is_txn_rejected(&mut self, txn: &Txn) -> Result<u128> {
        if let Some(txn_record_rejected) = self.get_txn_record_rejected(&txn.txn_id) {
            Ok(txn_record_rejected.txn_rejected_timestamp)
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    /// Purge rejected transactions.
    pub fn purge_txn_rejected(&mut self) -> Result<()> {
        self.get().and_then(|mut map| {
            map.rejected.clear();
            Some(())
        });
        Ok(())
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
    /// assert_eq!(1, lrmempooldb.size().0);
    /// ```
    pub fn size(&self) -> (usize, usize, usize) {
        if let Some(map) = self.get() {
            (map.pending.len(), map.validated.len(), map.rejected.len())
        } else {
            (0, 0, 0)
        }
    }

    /// Pushes changes to Reader.
    fn publish(&mut self) {
        self.write.publish();
    }
}
