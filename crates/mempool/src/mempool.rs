use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    result::Result as StdResult,
};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use primitives::TxHashString;
use serde::{Deserialize, Serialize};
use telemetry::{error, info, warn};
use tokio;
use vrrb_core::txn::{TransactionDigest, TxTimestamp, Txn};

use super::error::MempoolError;
use crate::create_tx_indexer;

pub type Result<T> = StdResult<T, MempoolError>;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub struct TxnRecord {
    pub txn_id: TxHashString,
    pub txn: Txn,
    pub status: TxnStatus,
    pub timestamp: TxTimestamp,
    pub added_timestamp: TxTimestamp,
    pub validated_timestamp: TxTimestamp,
    pub rejected_timestamp: TxTimestamp,
    pub deleted_timestamp: TxTimestamp,
}

impl TxnRecord {
    pub fn new(txn: Txn) -> TxnRecord {
        let added_timestamp = chrono::offset::Utc::now().timestamp();
        let timestamp = txn.timestamp();

        TxnRecord {
            txn_id: txn.id().to_string(),
            txn,
            timestamp,
            added_timestamp,
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

pub type PoolType = IndexMap<TxHashString, TxnRecord, FxBuildHasher>;

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxnStatus {
    #[default]
    Pending,
    Valdating,
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
    Add(TxnRecord),
    Remove(String),
}

impl Absorb<MempoolOp> for Mempool {
    fn absorb_first(&mut self, op: &mut MempoolOp, _: &Self) {
        match op {
            MempoolOp::Add(record) => {
                self.pool.insert(record.txn_id.clone(), record.clone());
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
        F: FnMut(&String, &mut TxnRecord) -> bool;
}

impl FetchFiltered for ReadHandle<Mempool> {
    fn fetch_filtered<F>(&self, amount: u32, f: F) -> Vec<TxnRecord>
    where
        F: FnMut(&String, &mut TxnRecord) -> bool,
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

    /// Adds a new transaction, makes sure it is unique in db.
    /// Pushes to the ReadHandle.
    #[deprecated(note = "use Self::insert instead")]
    pub fn add_txn(&mut self, txn: &Txn, _status: TxnStatus) -> Result<()> {
        self.insert(txn.to_owned())
    }

    pub fn insert(&mut self, txn: Txn) -> Result<()> {
        let txn_record = TxnRecord::new(txn);

        self.write
            .append(MempoolOp::Add(txn_record.to_owned()))
            .publish();

        tokio::spawn(async move {
            match create_tx_indexer(&txn_record).await {
                Ok(_) => {
                    info!("Successfully sent TxnRecord to block exploror indexer");
                },
                Err(e) => {
                    warn!("Error sending TxnRecord to block explorer indexer {}", e);
                },
            }
        });

        Ok(())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in
    /// db
    pub fn get_txn(&mut self, txn_hash: &TransactionDigest) -> Option<Txn> {
        if let Some(record) = self.get(txn_hash) {
            return Some(record.txn);
        }
        None
    }

    /// Getter for an entire pending Txn record
    pub fn get(&mut self, txn_hash: &TransactionDigest) -> Option<TxnRecord> {
        if txn_hash.is_empty() {
            return None;
        }

        self.pool().get(&txn_hash.to_string()).cloned()
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
    pub fn fetch_txns(&mut self, num_of_txns: usize) -> Vec<(TxHashString, TxnRecord)> {
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
        txn_batch: &HashSet<Txn>,
        _txns_status: TxnStatus,
    ) -> Result<()> {
        self.extend(txn_batch.clone())
    }

    pub fn extend(&mut self, txn_batch: HashSet<Txn>) -> Result<()> {
        txn_batch.into_iter().for_each(|t| {
            self.write.append(MempoolOp::Add(TxnRecord::new(t)));
        });

        self.publish();
        Ok(())
    }

    pub fn extend_with_records(&mut self, record_batch: HashSet<TxnRecord>) -> Result<()> {
        record_batch.into_iter().for_each(|t| {
            self.write.append(MempoolOp::Add(t));
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
    pub fn remove_txn(&mut self, txn: &Txn, _status: TxnStatus) -> Result<()> {
        self.remove(&txn.digest())
    }

    pub fn remove(&mut self, txn_hash: &TransactionDigest) -> Result<()> {
        self.write
            .append(MempoolOp::Remove(txn_hash.to_string()))
            .publish();
        Ok(())
    }

    /// Removes a batch of transactions, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    #[deprecated]
    pub fn remove_txn_batch(
        &mut self,
        txn_batch: &HashSet<Txn>,
        _txns_status: TxnStatus,
    ) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Remove(t.digest().to_string()));
        });

        self.publish();

        Ok(())
    }

    pub fn remove_txns(&mut self, txn_batch: &HashSet<TxHashString>) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Remove(t.to_string()));
        });

        self.publish();

        Ok(())
    }

    /// Was the Txn validated ? And when ?
    // TODO: rethink validated txn storage
    pub fn is_txn_validated(&mut self, txn: &Txn) -> Result<TxTimestamp> {
        match self.get(&txn.digest()) {
            Some(found) if matches!(found.status, TxnStatus::Validated) => {
                Ok(found.validated_timestamp)
            },
            _ => Err(MempoolError::TransactionNotFound(txn.digest())),
        }
    }

    /// Retrieves actual size of the mempooldb.
    pub fn size(&self) -> usize {
        self.pool().len()
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
    pub fn entries(&self) -> HashMap<TxHashString, TxnRecord> {
        self.handle()
            .values()
            .cloned()
            .map(|record| (record.txn_id.clone(), record))
            .collect()
    }

    /// Returns a vector of all transactions within the mempool
    pub fn values(&self) -> Vec<Txn> {
        self.handle()
            .values()
            .cloned()
            .map(|record| (record.txn))
            .collect()
    }
}
