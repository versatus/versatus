use std::{
    cmp::{Eq, PartialEq},
    collections::{HashMap, HashSet},
};

use evmap;
use vrrb_core::txn::Txn;

#[allow(clippy::derive_hash_xor_eq)]
#[derive(Hash)]
pub struct Validated(pub String, pub u128);

pub struct EvMemPool {
    pub tx_reader: evmap::ReadHandle<String, String>,
    pub tx_writer: evmap::WriteHandle<String, String>,
    pub cache: HashSet<Validated>,
}

impl Validated {
    pub fn new(tx_id: String, timestamp: u128) -> Validated {
        Validated(tx_id, timestamp)
    }
}

impl Default for EvMemPool {
    fn default() -> Self {
        let (r, w) = evmap::new();
        let cache = HashSet::new();
        EvMemPool {
            tx_reader: r,
            tx_writer: w,
            cache,
        }
    }
}

impl EvMemPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, tx_id: String, tx: String) {
        self.tx_writer.insert(tx_id, tx);
        self.publish();
    }

    pub fn add_batch(&mut self, txs: HashMap<String, String>) {
        txs.iter().for_each(|(k, v)| {
            self.tx_writer.insert(k.to_string(), v.to_string());
        });
        self.publish();
    }

    pub fn publish(&mut self) {
        self.tx_writer.refresh();
    }

    pub fn add_to_cache(&mut self, tx_id: String, timestamp: u128) {
        self.cache.insert(Validated::new(tx_id, timestamp));
    }

    pub fn check_cache(&self, tx_id: String) -> bool {
        self.cache.contains(&Validated(tx_id, 0))
    }

    pub fn convert_to_txn(&self, txn_id: String) -> Txn {
        Txn::from_string(&txn_id)
    }

    pub fn remove(&mut self, txn_id: String) {
        self.tx_writer.empty(txn_id);
        self.publish();
    }

    pub fn remove_batch(&mut self, txn_ids: HashSet<String>) {
        txn_ids.iter().for_each(|k| {
            self.tx_writer.empty(k.to_string());
        });
        self.publish();
    }
}

impl PartialEq for Validated {
    fn eq(&self, other: &Validated) -> bool {
        self.0 == other.0
    }
}

impl Eq for Validated {}
