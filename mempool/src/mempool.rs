
use std::{hash::Hash, collections::{HashSet}, time::{UNIX_EPOCH, SystemTime}};
use serde::{Serialize, Deserialize};

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
    pub txn_deleted_timestamp: u128
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
            txn_deleted_timestamp: 0
        }
    }
}

pub struct Mempool<K, V> where 
        K: Clone + Eq + Hash,
        V: Clone + Eq + Hash + evmap::ShallowCopy {

    pub txn_store_read: evmap::ReadHandle<K, V>,
    pub txn_store_write: evmap::WriteHandle<K, V>,
    pub db_creation: u128,
    pub last_operation: u128
}

pub type MempoolDB = Mempool<String, Box::<TxnRecord>>;

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
            last_operation: timestamp
        }
    }

    pub fn add_txn(&mut self, txn: &Txn) -> Result<(), MempoolError> {

        if ! self.txn_store_write.contains_key(&txn.txn_id) {
            self.txn_store_write.insert(txn.txn_id.clone(), Box::new(TxnRecord::new(txn)));
            self.push();
            Ok(())    
        } else {
            Err(MempoolError::TransactionExists)
        }
    }

    pub fn add_txn_batch(&mut self, txn_batch: HashSet<Txn>) -> Result<(), MempoolError> {

        txn_batch.iter().for_each(|t| {
            if ! self.txn_store_write.contains_key(&t.txn_id) {
                self.txn_store_write.insert(t.txn_id.clone(), Box::new(TxnRecord::new(t)));
            }
        });
        self.push();
        Ok(())
    }

    pub fn remove_txn_by_id(&mut self, txn_id: String) -> Result<(), MempoolError> {

        if self.txn_store_write.contains_key(&txn_id) {
            self.txn_store_write.empty(txn_id);
            self.push();
            Ok(())
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    pub fn remove_txn(&mut self, txn: &Txn) -> Result<(), MempoolError> {

        if self.txn_store_write.contains_key(&txn.txn_id) {
            self.txn_store_write.empty(txn.txn_id.clone());
            self.push();
            Ok(())
        } else {
            Err(MempoolError::TransactionMissing)
        }
    }

    pub fn remove_txn_batch(&mut self, txn_batch: HashSet<Txn>) -> Result<(), MempoolError> {

        txn_batch.iter().for_each(|t| {
            if self.txn_store_write.contains_key(&t.txn_id) {
                self.txn_store_write.empty(t.txn_id.clone());
            }
        });
        self.push();
        Ok(())
    }

    pub fn get_txn(&mut self, txn_id: String) -> Option<Txn> {
        if ! txn_id.is_empty() {
            self.txn_store_read.factory().handle().get_one(&txn_id).map(|t| Txn::from_string(&t.txn))
        }
        else {
            None
        }
    }

    pub fn validate(&mut self, _txn: &Txn) -> Result<(), MempoolError> {
        Ok(())
    }

    pub fn validate_by_id(&mut self, _txn_id: String) -> Result<(), MempoolError> {
        Ok(())        
    }

    pub fn push(&mut self) {
        self.txn_store_write.refresh();
        self.last_operation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
    }

    pub fn size(&self) -> usize {
        self.txn_store_write.len()
    }

}
