use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    result::Result as StdResult,
};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use serde::{Deserialize, Serialize};
use vrrb_core::transactions::{Transaction, TransactionDigest, TransactionKind, TxTimestamp};

use super::error::MempoolError;

pub type Result<T> = StdResult<T, MempoolError>;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Default, Deserialize)]
pub struct TxnRecord {
    pub txn_id: TransactionDigest,
    pub txn: TransactionKind,
    pub status: TxnStatus,
    pub timestamp: TxTimestamp,
    pub added_timestamp: TxTimestamp,
    pub validated_timestamp: TxTimestamp,
    pub rejected_timestamp: TxTimestamp,
    pub deleted_timestamp: TxTimestamp,
}

impl TxnRecord {
    pub fn new(txn: TransactionKind) -> TxnRecord {
        let added_timestamp = chrono::offset::Utc::now().timestamp();
        let timestamp = txn.timestamp();

        TxnRecord {
            txn_id: txn.id(),
            txn,
            timestamp,
            added_timestamp,
            ..Default::default()
        }
    }

    pub fn new_by_id(txn_id: &TransactionDigest) -> TxnRecord {
        TxnRecord {
            txn_id: txn_id.to_owned(),
            ..Default::default()
        }
    }
}

pub type PoolType = IndexMap<TransactionDigest, TxnRecord, FxBuildHasher>;

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxnStatus {
    #[default]
    Pending,
    Validating,
    Validated,
    Rejected,
}

/// Mempool stores unprocessed transactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mempool {
    pool: PoolType,
}

pub const DEFAULT_INITIAL_MEMPOOL_CAPACITY: usize = 10000;

impl Default for Mempool {
    fn default() -> Self {
        Mempool {
            pool: PoolType::with_capacity_and_hasher(
                DEFAULT_INITIAL_MEMPOOL_CAPACITY,
                <_>::default(),
            ),
        }
    }
}

impl Mempool {
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MempoolOp {
    Add(Box<TxnRecord>),
    Remove(TransactionDigest),
}

impl Absorb<MempoolOp> for Mempool {
    fn absorb_first(&mut self, op: &mut MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(record) => {
                self.pool.insert(record.txn_id.clone(), *record.clone());
            },
            MempoolOp::Remove(id) => {
                self.pool.remove(id);
            },
        }
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}

pub trait FetchFiltered {
    fn fetch_filtered<F>(&self, amount: u32, f: F) -> Vec<TxnRecord>
    where
        F: FnMut(&TransactionDigest, &mut TxnRecord) -> bool;
}

impl FetchFiltered for ReadHandle<Mempool> {
    fn fetch_filtered<F>(&self, amount: u32, f: F) -> Vec<TxnRecord>
    where
        F: FnMut(&TransactionDigest, &mut TxnRecord) -> bool,
    {
        if let Some(map) = self.enter().map(|guard| guard.clone()) {
            let mut result = map.pool;
            result.retain(f);
            let mut returned = Vec::<TxnRecord>::new();
            for (_, v) in &result {
                returned.push(v.clone());
            }
            // TODO:  Error - length
            return returned[0..amount as usize].to_vec();
        };
        Vec::<TxnRecord>::new()
    }
}

#[derive(Debug)]
pub struct LeftRightMempool {
    pub read: ReadHandle<Mempool>,
    pub write: WriteHandle<Mempool, MempoolOp>,
}

impl Default for LeftRightMempool {
    fn default() -> Self {
        let (write, read) = left_right::new::<Mempool, MempoolOp>();

        LeftRightMempool { read, write }
    }
}

impl LeftRightMempool {
    /// Creates new Mempool DB
    pub fn new() -> Self {
        Self::default()
    }

    /// Getter for Mempool DB
    pub fn pool(&self) -> PoolType {
        self.read
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
            .pool
    }

    /// Getter for Mempool DB
    #[deprecated]
    pub fn handle(&self) -> Option<Mempool> {
        self.read.enter().map(|guard| guard.clone())
    }

    /// Returns a new MempoolReadHandleFactory, to simplify multithread access.
    pub fn factory(&self) -> MempoolReadHandleFactory {
        let factory = self.read.factory();

        MempoolReadHandleFactory { factory }
    }

    /// Return the number of key-value pairs in the map.
    ///
    pub fn len(&self) -> usize {
        self.pool().len()
    }

    pub fn is_empty(&self) -> bool {
        self.pool().is_empty()
    }

    /// Adds a new transaction, makes sure it is unique in db.
    /// Pushes to the ReadHandle.
    #[deprecated(note = "use Self::insert instead")]
    pub fn add_txn(&mut self, txn: &TransactionKind, _status: TxnStatus) -> Result<()> {
        self.insert(txn.to_owned())?;
        Ok(())
    }

    pub fn insert(&mut self, txn: TransactionKind) -> Result<usize> {
        let txn_record = TxnRecord::new(txn);
        self.write
            .append(MempoolOp::Add(Box::new(txn_record)))
            .publish();

        Ok(self.size_in_kilobytes())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in
    /// db
    pub fn get_txn(&mut self, txn_hash: &TransactionDigest) -> Option<TransactionKind> {
        if let Some(record) = self.get(txn_hash) {
            return Some(record.txn);
        }
        None
    }

    /// Getter for an entire pending Txn record
    pub fn get(&mut self, txn_id: &TransactionDigest) -> Option<TxnRecord> {
        if txn_id.to_string().is_empty() {
            return None;
        }

        self.pool().get(txn_id).cloned()
    }

    /// It fetches the transactions from the pool and returns them.
    ///
    /// Arguments:
    ///
    /// * `num_of_txns`: The number of transactions to fetch from the pool.
    ///
    /// Returns:
    ///
    /// A vector of tuples of type (TxHashString, TxnRecord)
    pub fn fetch_txns(&mut self, num_of_txns: usize) -> Vec<(TransactionDigest, TxnRecord)> {
        let mut txns_records = vec![];
        for i in 0..num_of_txns {
            if i == self.pool().len() {
                break;
            }
            if let Some(txn_data) = self.pool().pop() {
                txns_records.push(txn_data);
            }
        }
        txns_records
    }

    /// Adds a batch of new transaction, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    #[deprecated(note = "use extend instead")]
    pub fn add_txn_batch(
        &mut self,
        txn_batch: &HashSet<TransactionKind>,
        _txns_status: TxnStatus,
    ) -> Result<()> {
        self.extend(txn_batch.clone())
    }

    pub fn extend(&mut self, txn_batch: HashSet<TransactionKind>) -> Result<()> {
        txn_batch.into_iter().for_each(|t| {
            self.write
                .append(MempoolOp::Add(Box::new(TxnRecord::new(t))));
        });

        self.publish();
        Ok(())
    }

    pub fn extend_with_records(&mut self, record_batch: HashSet<TxnRecord>) -> Result<()> {
        record_batch.into_iter().for_each(|t| {
            self.write.append(MempoolOp::Add(Box::new(t)));
        });

        self.publish();
        Ok(())
    }

    /// Removes a single transaction identified by id, makes sure it exists in
    /// db. Pushes to the ReadHandle.
    #[deprecated]
    pub fn remove_txn_by_id(&mut self, txn_hash: &TransactionDigest) -> Result<()> {
        self.remove(txn_hash)
    }

    /// Removes a single transaction identified by itself, makes sure it exists
    /// in db. Pushes to the ReadHandle.
    #[deprecated]
    pub fn remove_txn(&mut self, txn: &TransactionKind, _status: TxnStatus) -> Result<()> {
        self.remove(&txn.id())
    }

    pub fn remove(&mut self, id: &TransactionDigest) -> Result<()> {
        self.write
            .append(MempoolOp::Remove(id.to_owned()))
            .publish();
        Ok(())
    }

    /// Removes a batch of transactions, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    #[deprecated]
    pub fn remove_txn_batch(
        &mut self,
        txn_batch: &HashSet<TransactionKind>,
        _txns_status: TxnStatus,
    ) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Remove(t.id()));
        });

        self.publish();

        Ok(())
    }

    pub fn remove_txns(&mut self, txn_batch: &HashSet<TransactionDigest>) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Remove(t.to_owned()));
        });

        self.publish();

        Ok(())
    }

    /// Was the Txn validated ? And when ?
    // TODO: rethink validated txn storage
    pub fn is_txn_validated(&mut self, txn: &TransactionKind) -> Result<TxTimestamp> {
        match self.get(&txn.id()) {
            Some(found) if matches!(found.status, TxnStatus::Validated) => {
                Ok(found.validated_timestamp)
            },
            _ => Err(MempoolError::TransactionNotFound(txn.id())),
        }
    }

    /// Retrieves actual size of the mempooldb.
    pub fn size(&self) -> usize {
        self.pool().len()
    }

    /// Retrieves actual size of the mempooldb in Kilobytes.
    pub fn size_in_kilobytes(&self) -> usize {
        let txn_size_factor = std::mem::size_of::<TransactionKind>();
        let mempool_items = self.size();

        (mempool_items * txn_size_factor) / 1024
    }

    /// Pushes changes to Reader.
    fn publish(&mut self) {
        self.write.publish();
    }
}

impl From<PoolType> for LeftRightMempool {
    fn from(pool: PoolType) -> Self {
        let (write, read) = left_right::new::<Mempool, MempoolOp>();
        let mut mempool_db = Self { read, write };

        let records = pool.values().cloned().collect::<HashSet<TxnRecord>>();

        mempool_db.extend_with_records(records).unwrap_or_default();

        mempool_db
    }
}

impl Clone for LeftRightMempool {
    fn clone(&self) -> Self {
        Self::from(self.pool())
    }
}

#[derive(Clone, Debug)]
pub struct MempoolReadHandleFactory {
    factory: ReadHandleFactory<Mempool>,
}

impl MempoolReadHandleFactory {
    pub fn handle(&self) -> PoolType {
        self.factory
            .handle()
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
            .pool
    }

    /// Returns a hash map of all the key value pairs within the mempool
    pub fn entries(&self) -> HashMap<TransactionDigest, TxnRecord> {
        self.handle()
            .values()
            .cloned()
            .map(|record| (record.txn_id.clone(), record))
            .collect()
    }

    /// Returns a vector of all transactions within the mempool
    pub fn values(&self) -> Vec<TransactionKind> {
        self.handle()
            .values()
            .cloned()
            .map(|record| (record.txn))
            .collect()
    }

    pub fn get(&self, digest: &TransactionDigest) -> Option<TxnRecord> {
        if let Some(record) = self.handle().get(digest) {
            return Some(record.clone());
        }
        None
    }
}
