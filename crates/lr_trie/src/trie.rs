use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use crate::InnerTrieWrapper;
use crate::Operation;
use keccak_hash::H256;
pub use left_right::ReadHandleFactory;
use left_right::{ReadHandle, WriteHandle};
use patriecia::{db::Database, inner::InnerTrie, trie::Trie};
use serde::{Deserialize, Serialize};

/// Concurrent generic Merkle Patricia Trie
#[derive(Debug)]
pub struct LeftRightTrie<'a, K, V, D>
where
    D: Database,
    K: Serialize + Deserialize<'a>,
    V: Serialize + Deserialize<'a>,
{
    pub read_handle: ReadHandle<InnerTrie<D>>,
    pub write_handle: WriteHandle<InnerTrie<D>, Operation>,
    _marker: PhantomData<(K, V, &'a ())>,
}

impl<'a, D, K, V> LeftRightTrie<'a, K, V, D>
where
    D: Database,
    K: Serialize + Deserialize<'a>,
    V: Serialize + Deserialize<'a>,
{
    pub fn new(db: Arc<D>) -> Self {
        let (write_handle, read_handle) = left_right::new_from_empty(InnerTrie::new(db));

        Self {
            read_handle,
            write_handle,
            _marker: PhantomData,
        }
    }

    // pub fn handle(&self) -> InnerTrie<D> {
    //     self.read_handle
    //         .enter()
    //         .map(|guard| guard.clone())
    //         .unwrap_or_default()
    // }

    fn handle(&self) -> InnerTrieWrapper<D> {
        let read_handle = self
            .read_handle
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        InnerTrieWrapper::new(read_handle)
    }

    /// Returns a vector of all entries within the trie
    pub fn entries(&self) -> Vec<(K, V)> {
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

    #[deprecated(note = "replaced by insert")]
    pub fn add(&mut self, key: K, value: V) {
        self.insert(key, value)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.insert_uncommitted(key, value);
        self.publish();
    }

    pub fn extend(&mut self, values: Vec<(K, V)>) {
        self.extend_uncommitted(values);
        self.publish();
    }

    #[deprecated(note = "replaced by insert_uncommitted")]
    pub fn add_uncommitted(&mut self, key: K, value: V) {
        self.insert_uncommitted(key, value)
    }

    pub fn insert_uncommitted(&mut self, key: K, value: V) {
        //TODO: revisit the serializer used to store things on the trie
        let key = bincode::serialize(&key).unwrap_or_default();
        let value = bincode::serialize(&value).unwrap_or_default();
        self.write_handle.append(Operation::Add(key, value));
    }

    // pub fn extend_uncommitted<T>(&mut self, values: Vec<(K, V)>) {
    pub fn extend_uncommitted(&mut self, values: Vec<(K, V)>) {
        let mapped = values
            .into_iter()
            .map(|(key, value)| {
                //TODO: revisit the serializer used to store things on the trie
                let key = bincode::serialize(&key).unwrap_or_default();
                let value = bincode::serialize(&value).unwrap_or_default();

                (key, value)
            })
            .collect();

        self.write_handle.append(Operation::Extend(mapped));
    }
}

impl<'a, D, K, V> PartialEq for LeftRightTrie<'a, K, V, D>
where
    D: Database,
    K: Serialize + Deserialize<'a>,
    V: Serialize + Deserialize<'a>,
{
    fn eq(&self, other: &Self) -> bool {
        self.handle().root_hash() == other.handle().root_hash()
    }
}

impl<'a, D, K, V> Default for LeftRightTrie<'a, K, V, D>
where
    D: Database,
    K: Serialize + Deserialize<'a>,
    V: Serialize + Deserialize<'a>,
{
    fn default() -> Self {
        let (write_handle, read_handle) = left_right::new::<InnerTrie<D>, Operation>();
        Self {
            read_handle,
            write_handle,
            _marker: PhantomData,
        }
    }
}

impl<'a, D, K, V> From<D> for LeftRightTrie<'a, K, V, D>
where
    D: Database,
    K: Serialize + Deserialize<'a>,
    V: Serialize + Deserialize<'a>,
{
    fn from(db: D) -> Self {
        let db = Arc::new(db);
        let (write_handle, read_handle) = left_right::new_from_empty(InnerTrie::new(db));

        Self {
            read_handle,
            write_handle,
            _marker: PhantomData,
        }
    }
}

impl<'a, D, K, V> Clone for LeftRightTrie<'a, K, V, D>
where
    D: Database,
    K: Serialize + Deserialize<'a>,
    V: Serialize + Deserialize<'a>,
{
    fn clone(&self) -> Self {
        let db = self.handle().db();

        LeftRightTrie::new(db)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use patriecia::db::MemoryDB;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
    struct CustomValue {
        pub data: usize,
    }

    #[test]
    fn should_store_arbitrary_values() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut trie = LeftRightTrie::new(memdb);

        trie.insert("abcdefg", CustomValue { data: 100 });

        let value: CustomValue = trie.handle().get(&String::from("abcdefg")).unwrap();

        assert_eq!(value, CustomValue { data: 100 });
    }

    #[test]
    fn should_be_read_concurrently() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut trie = LeftRightTrie::new(memdb);

        trie.insert("abcdefg", CustomValue { data: 12345 });
        trie.insert("hijkl", CustomValue { data: 678910 });
        trie.insert("mnopq", CustomValue { data: 1112131415 });

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
