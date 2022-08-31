use crate::op::Bytes;
use crate::Operation;
use keccak_hash::H256;
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use patriecia::trie::Trie;
use patriecia::{db::Database, inner::InnerTrie};
use std::{fmt::Debug, sync::Arc};

/// Concurrent generic Merkle Patricia Trie
#[derive(Debug)]
pub struct LeftRightTrie<'a, D: Database> {
    pub read_handle: ReadHandle<InnerTrie<D>>,
    pub write_handle: WriteHandle<InnerTrie<D>, Operation<'a>>,
}

impl<'a, D: Database> LeftRightTrie<'a, D> {
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

    // TODO: revisit and consider if it's worth having it vs a simple iter over the inner trie
    // pub fn leaves(&self) -> Option<Vec<H::Hash>> {
    //     self.get().leaves()
    // }

    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<D>> {
        self.read_handle.factory()
    }

    pub fn publish(&mut self) {
        self.write_handle.publish();
    }

    pub fn add(&mut self, key: &'a Bytes, value: &'a Bytes) {
        self.write_handle.append(Operation::Add(key, value));
        self.publish();
    }

    // TODO: revisit once inner trie is refactored into patriecia
    pub fn extend(&mut self, values: Vec<(&'a Bytes, &'a Bytes)>) {
        self.write_handle.append(Operation::Extend(values));
        self.publish();
    }

    pub fn add_uncommitted(&mut self, key: &'a Bytes, value: &'a Bytes) {
        self.write_handle.append(Operation::Add(key, value));
    }

    pub fn extend_uncommitted(&mut self, values: Vec<(&'a Bytes, &'a Bytes)>) {
        self.write_handle.append(Operation::Extend(values));
    }
}

impl<'a, D: Database> PartialEq for LeftRightTrie<'a, D> {
    fn eq(&self, other: &Self) -> bool {
        self.get().root_hash() == other.get().root_hash()
    }
}

impl<'a, D: Database> Default for LeftRightTrie<'a, D> {
    fn default() -> Self {
        let (write_handle, read_handle) = left_right::new::<InnerTrie<D>, Operation>();
        Self {
            read_handle,
            write_handle,
        }
    }
}

impl<'a, D> Absorb<Operation<'a>> for InnerTrie<D>
where
    D: Database,
{
    fn absorb_first(&mut self, operation: &mut Operation<'a>, _other: &Self) {
        match operation {
            // TODO: report errors via instrumentation
            Operation::Add(key, value) => {
                self.insert(key, value).unwrap_or_default();
                self.commit().unwrap_or_default();
            }
            Operation::Remove(key) => {
                self.remove(key).unwrap_or_default();
            }
            Operation::Extend(values) => {
                // TODO: temp hack to get this going. Refactor ASAP
                for (k, v) in values {
                    self.insert(k, v).unwrap_or_default();
                }
                self.commit().unwrap_or_default();
            }
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

        trie.add(b"abcdefg", b"12345");
        trie.add(b"hijkl", b"678910");
        trie.add(b"mnopq", b"1112131415");

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
//         // let (write_handle, read_handle) = left_right::new::<InnerTrie<D>, Operation>();
//
//         let (write_handle, read_handle) = left_right::new_from_empty(InnerTrie::new(db));
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
