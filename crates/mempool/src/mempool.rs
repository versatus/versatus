use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    result::Result as StdResult,
    time::{SystemTime, UNIX_EPOCH},
};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use serde::{Deserialize, Serialize};
use state::state::NetworkState;
use txn::txn::{Transaction, Txn};

use super::error::MempoolError;

pub type Result<T> = StdResult<T, MempoolError>;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub struct TxnRecord {
    pub txn_id: String,
    pub txn: String,
    pub txn_added_timestamp: u128,
    pub txn_validated_timestamp: u128,
    pub txn_rejected_timestamp: u128,
    pub txn_deleted_timestamp: u128,
}

impl TxnRecord {
    pub fn new(txn: &Transaction) -> TxnRecord {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("The system time seems to be set to be earlier than 1970-01-01 00:00:00 UTC")
            .as_nanos();


        TxnRecord {
            txn_id: txn.get_id(),
            txn: txn.to_string(),
            txn_added_timestamp: timestamp,
            ..Default::default()
        }
    }

    pub fn new_by_id(txn_id: &str) -> TxnRecord {
        TxnRecord {
            txn_id: txn_id.to_owned(),
            ..Default::default()
        }
    }
}

impl Default for TxnRecord {
    fn default() -> Self {
        TxnRecord {
            txn_id: String::from(""),
            txn: String::from(""),
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
                },
                TxnStatus::Validated => {
                    self.validated
                        .insert(recdata.txn_id.clone(), recdata.clone());
                },
                TxnStatus::Rejected => {
                    self.rejected
                        .insert(recdata.txn_id.clone(), recdata.clone());
                },
            },
            MempoolOp::Remove(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.remove(&recdata.txn_id);
                },
                TxnStatus::Validated => {
                    self.validated.remove(&recdata.txn_id);
                },
                TxnStatus::Rejected => {
                    self.rejected.remove(&recdata.txn_id);
                },
            },
        }
    }

    fn absorb_second(&mut self, op: MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.insert(recdata.txn_id.clone(), recdata);
                },
                TxnStatus::Validated => {
                    self.validated.insert(recdata.txn_id.clone(), recdata);
                },
                TxnStatus::Rejected => {
                    self.rejected.insert(recdata.txn_id.clone(), recdata);
                },
            },
            MempoolOp::Remove(recdata, status) => match status {
                TxnStatus::Pending => {
                    self.pending.remove(&recdata.txn_id);
                },
                TxnStatus::Validated => {
                    self.validated.remove(&recdata.txn_id);
                },
                TxnStatus::Rejected => {
                    self.rejected.remove(&recdata.txn_id);
                },
            },
        }
    }

    fn drop_first(self: Box<Self>) {}

    fn drop_second(self: Box<Self>) {}

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}

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
            let mut result = map.pending;
            result.retain(f);
            let mut returned = Vec::<TxnRecord>::new();
            for (_, v) in &result {
                returned.push(v.clone());
            }
            // TODO:  Error - length
            if amount > returned.len() as u32 {
                return returned.to_vec();
            }
            return returned[0..amount as usize].to_vec();
        };
        Vec::<TxnRecord>::new()
    }
}

pub struct LeftRightMemPoolDB {
    pub read: ReadHandle<Mempool>,
    pub write: WriteHandle<Mempool, MempoolOp>,
}

impl Default for LeftRightMemPoolDB {
    fn default() -> Self {
        let (write, read) = left_right::new::<Mempool, MempoolOp>();

        LeftRightMemPoolDB { read, write }
    }
}

impl LeftRightMemPoolDB {
    /// Creates new Mempool DB
    pub fn new() -> Self {
        Self::default()
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
    /// use std::collections::HashMap;
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    ///
    /// let txn = Transaction::default();
    ///
    /// match lrmempooldb.add_txn(&txn, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// assert_eq!(1, lrmempooldb.size().0);
    /// ```
    pub fn add_txn(&mut self, txn: &Transaction, status: TxnStatus) -> Result<()> {
        self.write
            .append(MempoolOp::Add(TxnRecord::new(txn), status))
            .publish();
        Ok(())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in
    /// db. Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::{HashMap, HashSet};
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    /// let mut txns = HashSet::<Transaction>::new();
    ///
    /// txns.insert(Transaction::default());
    ///
    /// match lrmempooldb.add_txn_batch(&txns, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// if let Some(txn) = lrmempooldb.get_txn(&txns.iter().next().unwrap().get_id()) {
    ///     assert_eq!(1, lrmempooldb.size().0);
    /// } else {
    ///     panic!("Transaction missing !");
    /// };
    /// ```
    pub fn get_txn(&mut self, txn_id: &String) -> Option<Transaction> {
        if txn_id.is_empty() {
            return None;
        }

        self.get().and_then(|map| {
            map.pending
                .get(txn_id)
                .map(|t| Transaction::from_string(&t.txn))
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
    /// use std::collections::{HashMap, HashSet};
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    /// let mut txns = HashSet::<Transaction>::new();
    ///
    /// txns.insert(Transaction::default());
    ///
    /// match lrmempooldb.add_txn_batch(&txns, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// assert_eq!(1, lrmempooldb.size().0);
    /// ```
    pub fn add_txn_batch(
        &mut self,
        txn_batch: &HashSet<Transaction>,
        txns_status: TxnStatus,
    ) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write
                .append(MempoolOp::Add(TxnRecord::new(t), txns_status.clone()));
        });
        self.publish();
        Ok(())
    }

    /// Removes a single transaction identified by id, makes sure it exists in
    /// db. Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::{HashMap, HashSet};
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    /// let mut txns = HashSet::<Transaction>::new();
    ///
    /// txns.insert(Transaction::default());
    ///
    /// match lrmempooldb.add_txn_batch(&txns, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// match lrmempooldb.remove_txn_by_id(txns.iter().next().unwrap().get_id()) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
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

    /// Removes a single transaction identified by itself, makes sure it exists
    /// in db. Pushes to the ReadHandle.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::{HashMap, HashSet};
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    /// let txn_id = String::from("1");
    ///
    /// let txn = Transaction::default();
    ///
    /// match lrmempooldb.add_txn(&txn, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    /// match lrmempooldb.remove_txn(&txn, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// assert_eq!(0, lrmempooldb.size().0);
    /// ```
    pub fn remove_txn(&mut self, txn: &Transaction, status: TxnStatus) -> Result<()> {
        self.write
            .append(MempoolOp::Remove(TxnRecord::new(txn), status))
            .publish();
        Ok(())
    }

    /// Removes a batch of transactions, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::{HashMap, HashSet};
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    /// let mut txns = HashSet::<Transaction>::new();
    ///
    /// txns.insert(Transaction::default());
    ///
    /// match lrmempooldb.add_txn_batch(&txns, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// match lrmempooldb.remove_txn_batch(&txns, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
    /// };
    ///
    /// assert_eq!(0, lrmempooldb.size().0);
    /// ```
    // TODO: fix docs
    pub fn remove_txn_batch(
        &mut self,
        txn_batch: &HashSet<Transaction>,
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
        if let Some(mut map) = self.get() {
            map.rejected.clear()
        }

        Ok(())
    }

    /// Retrieves actual size of the mempooldb.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::{HashMap, HashSet};
    ///
    /// use mempool::mempool::{LeftRightMemPoolDB, TxnStatus};
    /// use txn::txn::Transaction;
    ///
    /// let mut lrmempooldb = LeftRightMemPoolDB::new();
    /// let mut txns = HashSet::<Transaction>::new();
    ///
    /// txns.insert(Transaction::default());
    ///
    /// match lrmempooldb.add_txn_batch(&txns, TxnStatus::Pending) {
    ///     Ok(_) => {},
    ///     Err(_) => {},
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
