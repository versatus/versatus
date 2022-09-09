use std::{fmt::Debug, sync::Arc};

use keccak_hash::H256;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use patriecia::{db::Database, inner::InnerTrie, trie::Trie};

use crate::Operation;

/// Concurrent generic Merkle Patricia Trie
#[derive(Debug)]
pub struct LeftRightTrie<D: Database> {
    pub read_handle: ReadHandle<InnerTrie<D>>,
    pub write_handle: WriteHandle<InnerTrie<D>, Operation>,
}

impl<D: Database> LeftRightTrie<D> {
    pub fn new(db: Arc<D>) -> Self {
        let (write_handle, read_handle) = left_right::new_from_empty(InnerTrie::new(db));

        Self {
            read_handle,
            write_handle,
        }
    }

    // TODO: consider renaming to handle, get_handle or get_read_handle
    pub fn get(&self) -> InnerTrie<D> {
        self.read_handle
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.get().len()
    }

    pub fn is_empty(&self) -> bool {
        self.get().len() == 0
    }

    pub fn root(&self) -> Option<H256> {
        self.get().root_hash().ok()
    }

    // TODO: revisit and consider if it's worth having it vs a simple iter over the
    // inner trie pub fn leaves(&self) -> Option<Vec<H::Hash>> {
    //     self.get().leaves()
    // }

    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<D>> {
        self.read_handle.factory()
    }

    pub fn publish(&mut self) {
        self.write_handle.publish();
    }

    pub fn add(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.write_handle.append(Operation::Add(key, value));
        self.publish();
    }

    // TODO: revisit once inner trie is refactored into patriecia
    pub fn extend(&mut self, values: Vec<(Vec<u8>, Vec<u8>)>) {
        self.write_handle.append(Operation::Extend(values));
        self.publish();
    }

    pub fn add_uncommitted(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.write_handle.append(Operation::Add(key, value));
    }

    pub fn extend_uncommitted(&mut self, values: Vec<(Vec<u8>, Vec<u8>)>) {
        self.write_handle.append(Operation::Extend(values));
    }
}

impl<D: Database> PartialEq for LeftRightTrie<D> {
    fn eq(&self, other: &Self) -> bool {
        self.get().root_hash() == other.get().root_hash()
    }
}

impl<D: Database> Default for LeftRightTrie<D> {
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
                // TODO: temp hack to get this going. Refactor ASAP
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

#[cfg(test)]
mod tests {
    use std::thread;

    use patriecia::db::MemoryDB;

    use super::*;

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
                let reader = trie.get();
                thread::spawn(move || {
                    // NOTE: 3 nodes plus the root add up to 4
                    assert_eq!(reader.len(), 4);
                })
            })
            .for_each(|handle| {
                handle.join().unwrap();
            });
    }
}

// TODO: revisit later
// impl<'a, E, D> From<E> for LeftRightTrie<'a, D>
// where
//     E: Iterator<Item = &'a Bytes>,
//     D: Database,
// {
//     fn from(values: E) -> Self {
//         // let (write_handle, read_handle) = left_right::new::<InnerTrie<D>,
// Operation>();
//
//         let (write_handle, read_handle) =
// left_right::new_from_empty(InnerTrie::new(db));
//
//         let mut trie = Self {
//             read_handle,
//             write_handle,
//         };
//
//         trie.extend(values.collect());
//
//         trie
//     }
// }
