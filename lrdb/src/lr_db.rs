use std::{cmp::Ordering, collections::HashMap, hash::Hash, time::SystemTime};

use crate::result::{LeftRightDbError, Result};
use lr_trie::db::Database;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};

/// Struct representing the LeftRight Database.
///
/// `ReadHandleFactory` provides a way of creating new ReadHandles to the database.
///
/// `WriteHandles` provides a way to gain write access to the database.
/// `last_refresh` denotes the lastest `refresh` of the database.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LeftRightDatabase<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug,
    V: Clone + Eq + evmap::ShallowCopy + std::fmt::Debug,
{
    r: evmap::ReadHandleFactory<K, V, ()>,
    w: evmap::WriteHandle<K, V, ()>,
    last_refresh: std::time::SystemTime,
}

impl<K, V> Database for LeftRightDatabase<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + std::fmt::Debug,
    V: Clone + Eq + evmap::ShallowCopy + Send + Sync + std::fmt::Debug,
{
    type Error = LeftRightDbError;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        todo!()
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<()> {
        todo!()
    }

    fn remove(&self, key: &[u8]) -> Result<()> {
        todo!()
    }

    fn flush(&self) -> Result<()> {
        todo!()
    }

    fn len(&self) -> Result<usize> {
        todo!()
    }

    fn is_empty(&self) -> Result<bool> {
        todo!()
    }
    //
}
