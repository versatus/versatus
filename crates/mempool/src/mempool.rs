use std::{collections::HashSet, hash::Hash, result::Result as StdResult};

use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use primitives::TxHashString;
use serde::{Deserialize, Serialize};
use vrrb_core::txn::{TxTimestamp, Txn};

use super::error::MempoolError;

pub type Result<T> = StdResult<T, MempoolError>;

//TODO: simplify mempool

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
        let timestamp = txn.timestamp;

        TxnRecord {
            txn_id: txn.digest(),
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
    Validated,
    Rejected,
}

/// Mempool stores unprocessed transactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mempool {
    pool: PoolType,
}

pub const DEFAULT_INITIAL_MEMPOOL_CAPACITY: usize = 100;

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
    Remove(TxHashString),
}

impl Absorb<MempoolOp> for Mempool {
    fn absorb_first(&mut self, op: &mut MempoolOp, _: &Self) {
        let txn_id = "9b6219bfefb10b913b3e0022f704f0fa0354a278ebf531889f9eb323bc699084".to_string();

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
            .clone()
    }

    /// Getter for Mempool DB
    #[deprecated]
    pub fn handle(&self) -> Option<Mempool> {
        self.read.enter().map(|guard| guard.clone())
    }

    /// Returns a new ReadHandleFactory, to simplify multithread access.
    pub fn factory(&self) -> ReadHandleFactory<Mempool> {
        self.read.factory()
    }

    /// Adds a new transaction, makes sure it is unique in db.
    /// Pushes to the ReadHandle.
    #[deprecated(note = "use Self::insert instead")]
    pub fn add_txn(&mut self, txn: &Txn, status: TxnStatus) -> Result<()> {
        self.insert(txn.to_owned())
    }

    pub fn insert(&mut self, txn: Txn) -> Result<()> {
        let mut txn_record = TxnRecord::new(txn);

        self.write.append(MempoolOp::Add(txn_record)).publish();
        Ok(())
    }

    /// Retrieves a single transaction identified by id, makes sure it exists in
    /// db
    pub fn get_txn(&mut self, txn_hash: &TxHashString) -> Option<Txn> {
        if let Some(record) = self.get(txn_hash) {
            return Some(record.txn);
        }
        None
    }

    /// Getter for an entire pending Txn record
    pub fn get(&mut self, txn_hash: &TxHashString) -> Option<TxnRecord> {
        if txn_hash.is_empty() {
            return None;
        }

        self.pool().get(txn_hash).cloned()
    }

    /// Adds a batch of new transaction, makes sure that each is unique in db.
    /// Pushes to ReadHandle after processing of the entire batch.
    #[deprecated(note = "use extend instead")]
    pub fn add_txn_batch(
        &mut self,
        txn_batch: &HashSet<Txn>,
        txns_status: TxnStatus,
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
    pub fn remove_txn_by_id(&mut self, txn_hash: TxHashString) -> Result<()> {
        self.remove(&txn_hash)
    }

    /// Removes a single transaction identified by itself, makes sure it exists
    /// in db. Pushes to the ReadHandle.
    #[deprecated]
    pub fn remove_txn(&mut self, txn: &Txn, status: TxnStatus) -> Result<()> {
        self.remove(&txn.digest())
    }

    pub fn remove(&mut self, txn_hash: &TxHashString) -> Result<()> {
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
        txns_status: TxnStatus,
    ) -> Result<()> {
        txn_batch.iter().for_each(|t| {
            self.write.append(MempoolOp::Remove(t.digest()));
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
