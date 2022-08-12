use crate::{inner::InnerTrie, Bytes, Operation};
use left_right::{ReadHandle, ReadHandleFactory, WriteHandle};
use rs_merkle::Hasher;
use std::fmt::Debug;

/// Concurrent generic Merkle Patricia Trie
pub struct LeftRightTrie<'a, H: Hasher> {
    pub read_handle: ReadHandle<InnerTrie<H>>,
    pub write_handle: WriteHandle<InnerTrie<H>, Operation<'a>>,
}

impl<'a, H: Hasher> LeftRightTrie<'a, H> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self) -> InnerTrie<H> {
        self.read_handle
            .enter()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.get().len()
    }

    pub fn root(&self) -> Option<H::Hash> {
        self.get().root()
    }

    pub fn leaves(&self) -> Option<Vec<H::Hash>> {
        self.get().leaves()
    }

    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<H>> {
        self.read_handle.factory()
    }

    pub fn publish(&mut self) {
        self.write_handle.publish();
    }

    pub fn add(&mut self, value: &'a Bytes) {
        self.write_handle.append(Operation::Add(value));
        self.publish();
    }

    pub fn extend(&mut self, values: Vec<&'a Bytes>) {
        self.write_handle.append(Operation::Extend(values));
        self.publish();
    }

    pub fn add_uncommitted(&mut self, value: &'a Bytes) {
        self.write_handle.append(Operation::Add(value));
    }

    pub fn extend_uncommitted(&mut self, values: Vec<&'a Bytes>) {
        self.write_handle.append(Operation::Extend(values));
    }
}

impl<'a, H: Hasher> PartialEq for LeftRightTrie<'a, H> {
    fn eq(&self, other: &Self) -> bool {
        self.get().root() == other.get().root()
    }
}

impl<'a, H: Hasher> Default for LeftRightTrie<'a, H> {
    fn default() -> Self {
        let (write_handle, read_handle) = left_right::new::<InnerTrie<H>, Operation>();
        Self {
            read_handle,
            write_handle,
        }
    }
}

impl<'a, H: Hasher> Debug for LeftRightTrie<'a, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: derive once MerkleTree impl Debug
        f.debug_struct("LeftRightTrie").finish()
    }
}

impl<'a, E, H> From<E> for LeftRightTrie<'a, H>
where
    E: Iterator<Item = &'a Bytes>,
    H: Hasher,
{
    fn from(values: E) -> Self {
        let (write_handle, read_handle) = left_right::new::<InnerTrie<H>, Operation>();

        let mut trie = Self {
            read_handle,
            write_handle,
        };

        trie.extend(values.collect());

        trie
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;
    use rs_merkle::algorithms::Sha256;

    #[test]
    fn should_be_read_concurrently() {
        let mut trie = LeftRightTrie::<Sha256>::new();

        trie.extend(vec!["abcdefg".as_bytes(), "hijkl".as_bytes()]);
        trie.add("abcdefg".as_bytes());

        // Spawn 10 threads and 10 readers that should report the exact same value
        [0..10]
            .iter()
            .map(|_| {
                let reader = trie.get();
                thread::spawn(move || {
                    assert_eq!(reader.len(), 3);
                })
            })
            .for_each(|handle| {
                handle.join().unwrap();
            });
    }
}
