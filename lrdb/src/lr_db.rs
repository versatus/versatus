use crate::result::{LeftRightDbError, Result};
use patriecia::db::Database;
use std::{hash::Hash, sync::Arc, time::SystemTime};

pub type Nonce = u32;

/// Struct representing the LeftRight Database.
///
/// `ReadHandleFactory` provides a way of creating new ReadHandles to the database.
///
/// `WriteHandles` provides a way to gain write access to the database.
/// `last_refresh` denotes the lastest `refresh` of the database.
// #[allow(dead_code)]
pub struct LeftRightDatabase<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug,
    V: Clone + Eq + evmap::ShallowCopy + std::fmt::Debug,
{
    r: evmap::ReadHandleFactory<K, V, ()>,
    w: evmap::WriteHandle<K, V, ()>,
    last_refresh: std::time::SystemTime,
}

impl<K, V> LeftRightDatabase<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug,
    V: Clone + Eq + evmap::ShallowCopy + std::fmt::Debug,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, V> Default for LeftRightDatabase<K, V>
where
    K: Clone + Eq + Hash + std::fmt::Debug,
    V: Clone + Eq + evmap::ShallowCopy + std::fmt::Debug,
{
    fn default() -> Self {
        let (vrrbdb_reader, mut vrrbdb_writer) = evmap::new();
        // TODO: revisit to figure out if this is really necessary
        // This is required to set up oplog
        // Otherwise there's no way to keep track of already inserted keys (before refresh)
        vrrbdb_writer.refresh();
        Self {
            r: vrrbdb_reader.factory(),
            w: vrrbdb_writer,
            last_refresh: SystemTime::now(),
        }
    }
}

// #[derive(Default, Debug, Clone)]
#[derive(Clone)]
// pub struct SyncDb<'a, K, V> {
pub struct SyncDb<'a> {
    db: Arc<LeftRightDatabase<&'a [u8], Vec<u8>>>,
}

impl<'a> SyncDb<'a> {
    pub fn new() -> Self {
        Self {
            db: Arc::new(LeftRightDatabase::new()),
        }
    }
}

impl<'a> Default for SyncDb<'a> {
    fn default() -> Self {
        Self {
            db: Arc::new(LeftRightDatabase::new()),
        }
    }
}

/*
//
// impl<K, V> Database for ArcWrap<LeftRightDatabase<K, V>>
// impl<K, V> Database for LeftRightDatabase<K, V>
// impl<K, V> Database for SyncDb<K, V>
impl<'a> Database for SyncDb<'a>
// where
//     K: Clone + Eq + Hash + Send + Sync + std::fmt::Debug,
//     V: Clone + Eq + evmap::ShallowCopy + Send + Sync + std::fmt::Debug,
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
*/
