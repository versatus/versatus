use std::{fmt::Debug, sync::Arc};

use crate::{Key, Operation};
use keccak_hash::H256;
use left_right::{Absorb, ReadHandle, WriteHandle};
use patriecia::{db::Database, inner::InnerTrie, trie::Trie};
use serde::{Deserialize, Serialize};

pub use left_right::ReadHandleFactory;

/// Concurrent generic Merkle Patricia Trie
#[derive(Debug)]
pub struct LeftRightTrie<D>
where
    D: Database,
{
    pub read_handle: ReadHandle<InnerTrie<D>>,
    pub write_handle: WriteHandle<InnerTrie<D>, Operation>,
}

impl<D> LeftRightTrie<D>
where
    D: Database,
{
    pub fn new(db: Arc<D>) -> Self {
        let (write_handle, read_handle) = left_right::new_from_empty(InnerTrie::new(db));

        Self {
            read_handle,
            write_handle,
        }
    }

    // TODO: consider renaming to handle, get_handle or get_read_handle
    #[deprecated(note = "Renamed to handle. This will be removed in later releases")]
    pub fn get(&self) -> InnerTrie<D> {
        self.handle()
    }

    pub fn handle(&self) -> InnerTrie<D> {
        self.read_handle
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Returns a vector of all entries within the trie
    pub fn entries<'a, T>(&self) -> Vec<(Key, T)>
    where
        T: Deserialize<'a> + Default,
    {
        todo!()
    }

    pub fn len(&self) -> usize {
        self.handle().iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.handle().len() == 0
    }

    pub fn root(&self) -> Option<H256> {
        self.handle().root_hash().ok()
    }

    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<D>> {
        self.read_handle.factory()
    }

    pub fn publish(&mut self) {
        self.write_handle.publish();
    }

    pub fn add<T>(&mut self, key: Key, value: T)
    where
        T: Serialize,
    {
        self.add_uncommitted(key, value);
        self.publish();
    }

    pub fn extend<T>(&mut self, values: Vec<(Key, T)>)
    where
        T: Serialize,
    {
        self.extend_uncommitted(values);
        self.publish();
    }

    pub fn add_uncommitted<T>(&mut self, key: Key, value: T)
    where
        T: Serialize,
    {
        //TODO: revisit the serializer used to store things on the trie
        let value = bincode::serialize(&value).unwrap_or_default();
        self.write_handle.append(Operation::Add(key, value));
    }

    pub fn extend_uncommitted<T>(&mut self, values: Vec<(Key, T)>)
    where
        T: Serialize,
    {
        let mapped = values
            .into_iter()
            .map(|(key, value)| {
                //TODO: revisit the serializer used to store things on the trie
                let value = bincode::serialize(&value).unwrap_or_default();

                (key, value)
            })
            .collect();

        self.write_handle.append(Operation::Extend(mapped));
    }
}

impl<D> PartialEq for LeftRightTrie<D>
where
    D: Database,
{
    fn eq(&self, other: &Self) -> bool {
        self.handle().root_hash() == other.handle().root_hash()
    }
}

impl<D> Default for LeftRightTrie<D>
where
    D: Database,
{
    fn default() -> Self {
        let (write_handle, read_handle) = left_right::new::<InnerTrie<D>, Operation>();
        Self {
            read_handle,
            write_handle,
        }
    }
}

impl<D> Absorb<Operation> for InnerTrie<D>
where
    D: Database,
{
    fn absorb_first(&mut self, operation: &mut Operation, _other: &Self) {
        match operation {
            // TODO: report errors via instrumentation
            Operation::Add(key, value) => {
                self.insert(key, value).unwrap_or_default();
                self.commit().unwrap_or_default();
            },
            Operation::Remove(key) => {
                self.remove(key).unwrap_or_default();
            },
            Operation::Extend(values) => {
                //
                // TODO: temp hack to get this going. Refactor ASAP
                //
                for (k, v) in values {
                    self.insert(k, v).unwrap_or_default();
                }
                self.commit().unwrap_or_default();
            },
        }
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}

impl<D: Database> From<D> for LeftRightTrie<D> {
    fn from(db: D) -> Self {
        let db = Arc::new(db);
        let (write_handle, read_handle) = left_right::new_from_empty(InnerTrie::new(db));

        Self {
            read_handle,
            write_handle,
        }
    }
}

impl<D: Database> Clone for LeftRightTrie<D> {
    fn clone(&self) -> Self {
        let db = self.handle().db();

        LeftRightTrie::new(db)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use patriecia::{db::MemoryDB, inner::TrieIterator};

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct CustomValue {
        pub data: usize,
    }

    #[test]
    fn should_store_arbitrary_values() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut trie = LeftRightTrie::new(memdb);

        trie.add(b"abcdefg".to_vec(), CustomValue { data: 100 });
        let value = trie.handle().get(b"abcdefg").unwrap().unwrap();
        let deserialized = bincode::deserialize::<CustomValue>(&value).unwrap();

        assert_eq!(deserialized, CustomValue { data: 100 });
    }

    #[test]
    fn should_be_read_concurrently() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut trie = LeftRightTrie::new(memdb);

        trie.add(b"abcdefg".to_vec(), b"12345".to_vec());
        trie.add(b"hijkl".to_vec(), b"678910".to_vec());
        trie.add(b"mnopq".to_vec(), b"1112131415".to_vec());

        // NOTE Spawn 10 threads and 10 readers that should report the exact same value
        [0..10]
            .iter()
            .map(|_| {
                let reader = trie.handle();
                thread::spawn(move || {
                    assert_eq!(reader.len(), 3);
                })
            })
            .for_each(|handle| {
                handle.join().unwrap();
            });
    }
}
