use keccak_hash::H256;
use lr_trie::{db::Database, Bytes, LeftRightTrie};
use std::{fmt::Debug, sync::Arc};

pub struct StateTrie<'a, D: Database> {
    trie: LeftRightTrie<'a, D>,
}

impl<'a, D: Database> StateTrie<'a, D> {
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
    ///
    ///  let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///  
    ///  state_trie.add(b"greetings", b"hello world");
    ///
    ///  assert_eq!(state_trie.len(), 1);
    /// ```
    ///
    pub fn add(&mut self, key: &'a Bytes, value: &'a Bytes) {
        self.trie.add(key, value);
    }

    /// Extends the state trie with the provided iterator over leaf values as bytes.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use std::sync::Arc;
    ///  use lr_trie::Bytes;
    ///
    ///  let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///
    ///  let vals: Vec<(&Bytes, &Bytes)> = vec![
    ///      (b"abcdefg", b"abcdefg"),
    ///      (b"hijkl", b"hijkl"),
    ///      (b"mnopq", b"mnopq"),
    ///  ];
    ///
    ///  state_trie.extend(vals);
    ///  assert_eq!(state_trie.len(), 2);
    /// ```
    ///
    pub fn extend(&mut self, values: Vec<(&'a Bytes, &'a Bytes)>) {
        self.trie.extend(values);
    }

    /// Returns the trie's Merkle root.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use std::sync::Arc;
    ///  use lr_trie::Bytes;
    ///
    ///  let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
    ///  let mut state_trie_a = StateTrie::new(memdb);
    ///
    ///  let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
    ///  let mut state_trie_b = StateTrie::new(memdb);
    ///
    ///  let vals: Vec<(&Bytes, &Bytes)> = vec![
    ///      (b"abcdefg", b"abcdefg"),
    ///      (b"hijkl", b"hijkl"),
    ///      (b"mnopq", b"mnopq"),
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
    ///
    ///  let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///
    ///  let vals: Vec<(&Bytes, &Bytes)> = vec![
    ///      (b"abcdefg", b"abcdefg"),
    ///      (b"hijkl", b"hijkl"),
    ///      (b"mnopq", b"mnopq"),
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
    ///  use std::sync::Arc;
    ///
    ///  let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
    ///  let mut state_trie = StateTrie::new(memdb);
    ///
    ///  assert_eq!(state_trie.len(), 0);
    /// ```
    ///
    pub fn is_empty(&self) -> bool {
        self.trie.len() == 0
    }
}

impl<'a, D: Database> PartialEq for StateTrie<'a, D> {
    fn eq(&self, other: &Self) -> bool {
        self.root() == other.root()
    }
}

impl<'a, D: Database> Debug for StateTrie<'a, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateTrie")
            .field("trie", &self.trie)
            .finish()
    }
}

// TODO: revisit once patriecia is implemented
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn new_creates_default_empty_trie() {
        let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
        let state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 1);
    }

    #[test]
    fn new_creates_trie_from_lrdb_values() {
        let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        state_trie.add(b"abcdefg", b"12345");
        state_trie.add(b"hijkl", b"1000");
        state_trie.add(b"mnopq", b"askskaskj");

        let root = state_trie.root().unwrap();
        let root = format!("0x{}", hex::encode(root));

        let target_root =
            "0xfcea4ea8a4decaf828666306c81977085ba9488d981c759ac899862fd4e9174e".to_string();

        assert_eq!(state_trie.len(), 4);
        assert_eq!(root, target_root);
    }

    #[test]
    fn should_add_node_to_trie() {
        let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 1);

        state_trie.add(b"greetings", b"hello world");

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 2);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 1);

        let vals: Vec<(&Bytes, &Bytes)> = vec![
            (b"abcdefg", b"abcdefg"),
            (b"hijkl", b"hijkl"),
            (b"mnopq", b"mnopq"),
        ];

        state_trie.extend(vals);

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));

        let mut state_trie_a = StateTrie::new(memdb.clone());
        let mut state_trie_b = StateTrie::new(memdb.clone());

        let vals: Vec<(&Bytes, &Bytes)> = vec![
            (b"abcdefg", b"abcdefg"),
            (b"hijkl", b"hijkl"),
            (b"mnopq", b"mnopq"),
        ];

        state_trie_a.extend(vals.clone());
        state_trie_b.extend(vals.clone());

        assert_eq!(state_trie_a, state_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let memdb = Arc::new(lr_trie::db::MemoryDB::new(true));

        let mut state_trie_a = StateTrie::new(memdb.clone());
        let mut state_trie_b = StateTrie::new(memdb.clone());

        let vals: Vec<(&Bytes, &Bytes)> = vec![
            (b"abcdefg", b"abcdefg"),
            (b"hijkl", b"hijkl"),
            (b"mnopq", b"mnopq"),
        ];

        state_trie_a.extend(vals.clone());
        state_trie_b.extend(vals.clone());
        state_trie_b.add(b"mnopq", b"bananas");

        assert_ne!(state_trie_a, state_trie_b);
    }
}
