use crate::verifiable::Verifiable;
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use std::cmp::Eq;
use std::hash::Hash;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pool<K: Serialize + Eq + Hash, V: Verifiable> {
    pub kind: PoolKind,
    pub pending: LinkedHashMap<K, V>,
    pub confirmed: LinkedHashMap<K, V>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PoolKind {
    Txn,
    Claim,
}

impl<K: Serialize + Eq + Hash, V: Verifiable> Pool<K, V> {
    pub fn new(kind: PoolKind) -> Pool<K, V> {
        match kind {
            PoolKind::Txn => Pool {
                kind,
                pending: LinkedHashMap::new(),
                confirmed: LinkedHashMap::new(),
            },
            PoolKind::Claim => Pool {
                kind,
                pending: LinkedHashMap::new(),
                confirmed: LinkedHashMap::new(),
            },
        }
    }
}
