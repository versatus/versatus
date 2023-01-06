use std::{
    cmp::{Eq, PartialEq},
    collections::{HashMap, HashSet},
};

use evmap::{self, ShallowCopy};
use primitives::{ByteVec, TxHashString};
use vrrb_core::txn::Txn;

#[allow(clippy::derive_hash_xor_eq)]
#[derive(Hash)]
pub struct Validated(pub String, pub u128);

pub struct EvMempool {
    tx_reader: evmap::ReadHandle<TxHashString, Txn>,
    tx_writer: evmap::WriteHandle<TxHashString, Txn>,
    cache: HashSet<Validated>,
}

impl Validated {
    pub fn new(tx_id: String, timestamp: u128) -> Validated {
        Validated(tx_id, timestamp)
    }
}

impl Default for EvMempool {
    fn default() -> Self {
        let (r, w) = evmap::new();
        let cache = HashSet::new();

        EvMempool {
            tx_reader: r,
            tx_writer: w,
            cache,
        }
    }
}

impl EvMempool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, txn_hash: TxHashString, txn: ByteVec) {
        self.tx_writer.insert(txn_hash, txn);
        self.publish();
    }

    pub fn add_batch(&mut self, txs: HashMap<TxHashString, ByteVec>) {
        txs.iter().for_each(|(k, v)| {
            self.tx_writer.insert(k.to_string(), v.to_vec());
        });
        self.publish();
    }

    pub fn publish(&mut self) {
        self.tx_writer.refresh();
    }

    pub fn add_to_cache(&mut self, tx_hash: TxHashString, timestamp: u128) {
        self.cache.insert(Validated::new(tx_hash, timestamp));
    }

    pub fn check_cache(&self, tx_hash: TxHashString) -> bool {
        self.cache.contains(&Validated(tx_hash, 0))
    }

    pub fn remove(&mut self, txn_hash: TxHashString) {
        self.tx_writer.empty(txn_hash);
        self.publish();
    }

    pub fn remove_batch(&mut self, txn_hashes: HashSet<TxHashString>) {
        txn_hashes.iter().for_each(|k| {
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
