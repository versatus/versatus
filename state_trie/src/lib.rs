use keccak_hash::H256;
use lr_trie::LeftRightTrie;
use patriecia::db::Database;
use std::{fmt::Debug, sync::Arc};

pub struct StateTrie<D: Database> {
    trie: LeftRightTrie<D>,
}

impl<D: Database> StateTrie<D> {
    /// Creates a new empty state trie.
    pub fn new(db: Arc<D>) -> Self {
        Self {
            trie: LeftRightTrie::new(db),
        }
    }

    /// Adds a single leaf value serialized to bytes
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use std::sync::Arc;
    ///  use patriecia::db::MemoryDB;
    ///
    ///  let memdb = Arc::new(MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///  
    ///  state_trie.add(b"greetings.to_vec()".to_vec(), b"hello world".to_vec());
    ///
    ///  assert_eq!(state_trie.len(), 1);
    /// ```
    ///
    pub fn add(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.trie.add(key, value);
    }

    /// Extends the state trie with the provided iterator over leaf values as bytes.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use std::sync::Arc;
    ///  use lr_trie::Bytes;
    ///  use patriecia::db::MemoryDB;
    ///
    ///  let memdb = Arc::new(MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///
    ///  let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///      (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///      (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///      (b"mnopq".to_vec(), b"mnopq".to_vec()),
    ///  ];
    ///
    ///  state_trie.extend(vals);
    ///  assert_eq!(state_trie.len(), 2);
    /// ```
    ///
    pub fn extend(&mut self, values: Vec<(Vec<u8>, Vec<u8>)>) {
        self.trie.extend(values);
    }

    /// Returns the trie's Merkle root.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use std::sync::Arc;
    ///  use lr_trie::Bytes;
    ///  use patriecia::db::MemoryDB;
    ///
    ///  let memdb = Arc::new(MemoryDB::new(true));
    ///  let mut state_trie_a = StateTrie::new(memdb);
    ///
    ///  let memdb = Arc::new(MemoryDB::new(true));
    ///  let mut state_trie_b = StateTrie::new(memdb);
    ///
    ///  let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///      (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///      (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///      (b"mnopq".to_vec(), b"mnopq".to_vec()),
    ///  ];
    ///
    ///  state_trie_a.extend(vals.clone());
    ///  state_trie_b.extend(vals.clone());
    ///
    ///  assert_eq!(state_trie_a.root(), state_trie_b.root());
    /// ```
    ///
    pub fn root(&self) -> Option<H256> {
        self.trie.root()
    }

    /// Returns the count of leaves in the state trie.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use std::sync::Arc;
    ///  use lr_trie::Bytes;
    ///  use patriecia::db::MemoryDB;
    ///
    ///  let memdb = Arc::new(MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///
    ///  let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///      (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///      (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///      (b"mnopq".to_vec(), b"mnopq".to_vec()),
    ///  ];
    ///
    ///  state_trie.extend(vals);
    ///
    ///  assert_eq!(state_trie.len(), 2);
    /// ```
    ///
    pub fn len(&self) -> usize {
        self.trie.len()
    }

    /// Returns true if there are no values in the trie.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use patriecia::db::MemoryDB;
    ///  use std::sync::Arc;
    ///
    ///  let memdb = Arc::new(MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///
    ///  assert_eq!(state_trie.len(), 0);
    /// ```
    ///
    pub fn is_empty(&self) -> bool {
        self.trie.len() == 0
    }
}

impl<D: Database> PartialEq for StateTrie<D> {
    fn eq(&self, other: &Self) -> bool {
        self.root() == other.root()
    }
}

impl<D: Database> Debug for StateTrie<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateTrie")
            .field("trie", &self.trie)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patriecia::db::MemoryDB;
    use std::sync::Arc;

    #[test]
    fn new_creates_default_empty_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 1);
    }

    #[test]
    fn new_creates_trie_from_lrdb_values() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        state_trie.add(b"abcdefg".to_vec(), b"12345".to_vec());
        state_trie.add(b"hijkl".to_vec(), b"1000".to_vec());
        state_trie.add(b"mnopq".to_vec(), b"askskaskj".to_vec());

        let root = state_trie.root().unwrap();
        let root = format!("0x{}", hex::encode(root));

        let target_root =
            "0xfcea4ea8a4decaf828666306c81977085ba9488d981c759ac899862fd4e9174e".to_string();

        assert_eq!(state_trie.len(), 4);
        assert_eq!(root, target_root);
    }

    #[test]
    fn should_add_node_to_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 1);

        state_trie.add(b"greetings".to_vec(), b"hello world".to_vec());

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 2);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 1);

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        state_trie.extend(vals);

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let memdb = Arc::new(MemoryDB::new(true));

        let mut state_trie_a = StateTrie::new(memdb.clone());
        let mut state_trie_b = StateTrie::new(memdb.clone());

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        state_trie_a.extend(vals.clone());
        state_trie_b.extend(vals.clone());

        assert_eq!(state_trie_a, state_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let memdb = Arc::new(MemoryDB::new(true));

        let mut state_trie_a = StateTrie::new(memdb.clone());
        let mut state_trie_b = StateTrie::new(memdb.clone());

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        state_trie_a.extend(vals.clone());
        state_trie_b.extend(vals.clone());
        state_trie_b.add(b"mnopq".to_vec(), b"bananas".to_vec());

        assert_ne!(state_trie_a, state_trie_b);
    }
}

// TODO: revisit once lrdb is integrated with tries
// impl<'a, D, E> From<E> for StateTrie<'a, H>
// where
//     D: Database,
//     E: Iterator<Item = &'a Bytes>,
// {
//     fn from(values: E) -> Self {
//         let trie = LeftRightTrie::from(values);
//         Self { trie }
//     }
// }
