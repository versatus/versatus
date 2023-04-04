#![allow(deprecated, deprecated_in_future)]
//FEATURE TAG(S): Left-Right Mempool, Validator Cores, Tx Validation, Tx Writes
// to Confirmed, Block Validation & Confirmation, Block Structure
use std::{cmp::Eq, hash::Hash};

/// This module declares and implements methods on Pools
//TODO: Replace this module with the Left-Right Pool(s).
use ritelinked::LinkedHashMap;
use serde::{Deserialize, Serialize};
use vrrb_core::verifiable::Verifiable;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[deprecated(note = "meant to be replaced with a Left-Right wrapped structure")]
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
