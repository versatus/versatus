use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    result::Result as StdResult,
};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use serde::{Deserialize, Serialize};
use vrrb_core::transactions::{TransactionDigest, TxTimestamp, Transaction};


use super::error::MempoolError;

pub type Result<T> = StdResult<T, MempoolError>;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Default)]
pub struct TxnRecord<T> {
    pub txn_id: TransactionDigest,
    pub txn: T,
    pub status: TxnStatus,
    pub timestamp: TxTimestamp,
    pub added_timestamp: TxTimestamp,
    pub validated_timestamp: TxTimestamp,
    pub rejected_timestamp: TxTimestamp,
    pub deleted_timestamp: TxTimestamp,
}

impl<T: for<'a> Transaction<'a>> TxnRecord<T> {
    pub fn new(txn: T) -> TxnRecord<T> {
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

    pub fn new_by_id(txn_id: &TransactionDigest) -> TxnRecord<T> {
        TxnRecord {
            txn_id: txn_id.to_owned(),
            ..Default::default()
        }
    }
}

pub type PoolType<T> = IndexMap<TransactionDigest, TxnRecord<T>, FxBuildHasher>;

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
pub struct Mempool<T> {
    pool: PoolType<T>,
}

pub const DEFAULT_INITIAL_MEMPOOL_CAPACITY: usize = 10000;

impl<T: for<'a> Transaction<'a>> Default for Mempool<T> {
    fn default() -> Self {
        Mempool {
            pool: PoolType::with_capacity_and_hasher(
                DEFAULT_INITIAL_MEMPOOL_CAPACITY,
                <_>::default(),
            ),
        }
    }
}

impl<T: for<'a> Transaction<'a>> Mempool<T> {
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MempoolOp<T> {
    Add(Box<TxnRecord<T>>),
    Remove(TransactionDigest),
}

impl<T: for<'a> Transaction<'a>> Absorb<MempoolOp<T>> for Mempool<T> {
    fn absorb_first(&mut self, op: &mut MempoolOp<T>, _: &Self) {
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

pub trait FetchFiltered<T: for<'a> Transaction<'a>> {
    fn fetch_filtered<F>(&self, amount: u32, f: F) -> Vec<TxnRecord<T>>
    where
        F: FnMut(&TransactionDigest, &mut TxnRecord<T>) -> bool;
}

impl<T: for<'a> Transaction<'a>> FetchFiltered<T> for ReadHandle<Mempool<T>> {
    fn fetch_filtered<F>(&self, amount: u32, f: F) -> Vec<TxnRecord<T>>
    where
        F: FnMut(&TransactionDigest, &mut TxnRecord<T>) -> bool,
    {
        if let Some(map) = self.enter().map(|guard| guard.clone()) {
            let mut result = map.pool;
            result.retain(f);
            let mut returned = Vec::<TxnRecord<T>>::new();
            for (_, v) in &result {
                returned.push(v.clone());
            }
            // TODO:  Error - length
            return returned[0..amount as usize].to_vec();
        };
        Vec::<TxnRecord<T>>::new()
    }
}

#[derive(Debug)]
pub struct LeftRightMempool<T: for<'a> Transaction<'a>> {
    pub read: ReadHandle<Mempool<T>>,
    pub write: WriteHandle<Mempool<T>, MempoolOp<T>>,
}

impl<T: for<'a> Transaction<'a>> Default for LeftRightMempool<T> {
    fn default() -> Self {
        let (write, read) = left_right::new::<Mempool<T>, MempoolOp<T>>();

        LeftRightMempool { read, write }
    }
}

impl<T: for<'a> Transaction<'a>> LeftRightMempool<T> {
    /// Creates new Mempool DB
    pub fn new() -> Self {
        Self::default()
    }

    /// Getter for Mempool DB
    pub fn pool(&self) -> PoolType<T> {
        self.read
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
            .pool
    }

    /// Getter for Mempool DB
    #[deprecated]
    pub fn handle(&self) -> Option<Mempool<T>> {
        self.read.enter().map(|guard| guard.clone())
    }

    /// Returns a new MempoolReadHandleFactory, to simplify multithread access.
    pub fn factory(&self) -> MempoolReadHandleFactory<T> {
        let factory = self.read.factory();

        MempoolReadHandleFactory { factory }
    }

    /// Adds a new transaction, makes sure it is unique in db.
    /// Pushes to the ReadHandle.
    #[deprecated(note = "use Self::insert instead")]
    pub fn add_txn(&mut self, txn: &T, _status: TxnStatus) -> Result<()> {
        self.insert(txn.to_owned())?;
        Ok(())
    }

    pub fn insert(&mut self, txn: T) -> Result<usize> {
        let txn_record = TxnRecord::new(txn);
        self.write
            .append(MempoolOp::Add(Box::new(txn_record)))
            .publish();

        Ok(self.size_in_kilobytes())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in
    /// db
    pub fn get_txn(&mut self, txn_hash: &TransactionDigest) -> Option<T> {
        if let Some(record) = self.get(txn_hash) {
            return Some(record.txn);
        }
        None
    }

    /// Getter for an entire pending Txn record
    pub fn get(&mut self, txn_id: &TransactionDigest) -> Option<TxnRecord<T>> {
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
    pub fn fetch_txns(&mut self, num_of_txns: usize) -> Vec<(TransactionDigest, TxnRecord<T>)> {
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
        txn_batch: &HashSet<T>,
        _txns_status: TxnStatus,
    ) -> Result<()> {
        self.extend(txn_batch.clone())
    }

    pub fn extend(&mut self, txn_batch: HashSet<T>) -> Result<()> {
        txn_batch.into_iter().for_each(|t| {
            self.write
                .append(MempoolOp::Add(Box::new(TxnRecord::new(t))));
        });

        self.publish();
        Ok(())
    }

    pub fn extend_with_records(&mut self, record_batch: HashSet<TxnRecord<T>>) -> Result<()> {
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
    pub fn remove_txn(&mut self, txn: &T, _status: TxnStatus) -> Result<()> {
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
        txn_batch: &HashSet<T>,
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
    pub fn is_txn_validated(&mut self, txn: &T) -> Result<TxTimestamp> {
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
        let txn_size_factor = std::mem::size_of::<T>();
        let mempool_items = self.size();

        (mempool_items * txn_size_factor) / 1024
    }

    /// Pushes changes to Reader.
    fn publish(&mut self) {
        self.write.publish();
    }
}

impl<T: Hash + Eq + for<'a> Transaction<'a>> From<PoolType<T>> for LeftRightMempool<T> {
    fn from(pool: PoolType<T>) -> Self {
        let (write, read) = left_right::new::<Mempool<T>, MempoolOp<T>>();
        let mut mempool_db = Self { read, write };

        let records = pool.values().cloned().collect::<HashSet<TxnRecord<T>>>();

        mempool_db.extend_with_records(records).unwrap_or_default();

        mempool_db
    }
}

impl<T: Hash + Eq + for<'a> Transaction<'a>> Clone for LeftRightMempool<T> {
    fn clone(&self) -> Self {
        Self::from(self.pool())
    }
}

#[derive(Clone, Debug)]
pub struct MempoolReadHandleFactory<T> {
    factory: ReadHandleFactory<Mempool<T>>,
}

impl<T: for<'a> Transaction<'a> > MempoolReadHandleFactory<T> {
    pub fn handle(&self) -> PoolType<T> {
        self.factory
            .handle()
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
            .pool
    }

    /// Returns a hash map of all the key value pairs within the mempool
    pub fn entries(&self) -> HashMap<TransactionDigest, TxnRecord<T>> {
        self.handle()
            .values()
            .cloned()
            .map(|record| (record.txn_id.clone(), record))
            .collect()
    }

    /// Returns a vector of all transactions within the mempool
    pub fn values(&self) -> Vec<T> {
        self.handle()
            .values()
            .cloned()
            .map(|record| (record.txn))
            .collect()
    }

    pub fn get(&self, digest: &TransactionDigest) -> Option<TxnRecord<T>> {
        if let Some(record) = self.handle().get(digest) {
            return Some(record.clone());
        }
        None
    }
}
