use std::{fmt::Debug, sync::Arc};

use lr_trie::LeftRightTrie;
use patriecia::{db::Database, H256};

pub struct TxTrie<D: Database> {
    trie: LeftRightTrie<D>,
}

#[deprecated(note = "Use lr_trie directly instead")]
impl<D: Database> TxTrie<D> {
    /// Creates a new empty tx trie.
    pub fn new(db: Arc<D>) -> Self {
        Self {
            trie: LeftRightTrie::new(db),
        }
    }

    /// Adds a single leaf value serialized to bytes
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use patriecia::db::MemoryDB;
    /// use tx_trie::TxTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut tx_trie = TxTrie::new(memdb);
    ///
    /// tx_trie.add(b"greetings.to_vec()".to_vec(), b"hello world".to_vec());
    ///
    /// assert_eq!(tx_trie.len(), 1);
    /// ```
    pub fn add(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.trie.add(key, value);
    }

    /// Extends the tx trie with the provided iterator over leaf values as
    /// bytes. Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use tx_trie::TxTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut tx_trie = TxTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// tx_trie.extend(vals);
    /// assert_eq!(tx_trie.len(), 3);
    /// ```
    pub fn extend(&mut self, values: Vec<(Vec<u8>, Vec<u8>)>) {
        self.trie.extend(values);
    }

    /// Returns the trie's Merkle root.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use tx_trie::TxTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut tx_trie_a = TxTrie::new(memdb);
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut tx_trie_b = TxTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// tx_trie_a.extend(vals.clone());
    /// tx_trie_b.extend(vals.clone());
    ///
    /// assert_eq!(tx_trie_a.root(), tx_trie_b.root());
    /// ```
    pub fn root(&self) -> Option<H256> {
        self.trie.root()
    }

    /// Returns the count of leaves in the tx trie.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use tx_trie::TxTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut tx_trie = TxTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// tx_trie.extend(vals);
    ///
    /// assert_eq!(tx_trie.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.trie.len()
    }

    /// Returns true if there are no values in the trie.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use patriecia::db::MemoryDB;
    /// use tx_trie::TxTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut tx_trie = TxTrie::new(memdb);
    ///
    /// assert_eq!(tx_trie.len(), 0);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.trie.len() == 0
    }
}

impl<D: Database> PartialEq for TxTrie<D> {
    fn eq(&self, other: &Self) -> bool {
        self.root() == other.root()
    }
}

impl<D: Database> Default for TxTrie<D> {
    fn default() -> Self {
        Self {
            trie: Default::default(),
        }
    }
}

impl<D: Database> Debug for TxTrie<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: derive once MerkleTree impl Debug
        f.debug_struct("TxTrie").finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use patriecia::db::MemoryDB;

    use super::*;

    #[test]
    fn new_creates_default_empty_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let tx_trie = TxTrie::new(memdb);

        assert!(tx_trie.root().is_some());
        assert_eq!(tx_trie.len(), 0);
    }

    #[test]
    fn new_creates_trie_from_lrdb_values() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut tx_trie = TxTrie::new(memdb);

        tx_trie.add(b"abcdefg".to_vec(), b"12345".to_vec());
        tx_trie.add(b"hijkl".to_vec(), b"1000".to_vec());
        tx_trie.add(b"mnopq".to_vec(), b"askskaskj".to_vec());

        let root = tx_trie.root().unwrap();
        let root = format!("0x{}", hex::encode(root));

        let target_root =
            "0xfcea4ea8a4decaf828666306c81977085ba9488d981c759ac899862fd4e9174e".to_string();

        assert_eq!(tx_trie.len(), 3);
        assert_eq!(root, target_root);
    }

    #[test]
    fn should_add_node_to_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut tx_trie = TxTrie::new(memdb);

        assert!(tx_trie.root().is_some());
        assert_eq!(tx_trie.len(), 0);

        tx_trie.add(b"greetings".to_vec(), b"hello world".to_vec());

        assert_ne!(tx_trie.root(), None);
        assert_eq!(tx_trie.len(), 1);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut tx_trie = TxTrie::new(memdb);

        assert!(tx_trie.root().is_some());
        assert_eq!(tx_trie.len(), 0);

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        tx_trie.extend(vals);

        assert_ne!(tx_trie.root(), None);
        assert_eq!(tx_trie.len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let memdb = Arc::new(MemoryDB::new(true));

        let mut tx_trie_a = TxTrie::new(memdb.clone());
        let mut tx_trie_b = TxTrie::new(memdb);

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        tx_trie_a.extend(vals.clone());
        tx_trie_b.extend(vals.clone());

        assert_eq!(tx_trie_a, tx_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let memdb = Arc::new(MemoryDB::new(true));

        let mut tx_trie_a = TxTrie::new(memdb.clone());
        let mut tx_trie_b = TxTrie::new(memdb.clone());

        let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
            (b"hijkl".to_vec(), b"hijkl".to_vec()),
            (b"mnopq".to_vec(), b"mnopq".to_vec()),
        ];

        tx_trie_a.extend(vals.clone());
        tx_trie_b.extend(vals.clone());
        tx_trie_b.add(b"mnopq".to_vec(), b"bananas".to_vec());

        assert_ne!(tx_trie_a, tx_trie_b);
    }
}

// TODO: revisit later once lrdb is integrated with tries
// impl<D, E> From<E> for TxTrie<D>
// where
//     D: Database,
//     E: Iterator<Item = Vec<u8>>,
// {
//     fn from(values: E) -> Self {
//         let trie = LeftRightTrie::from(values);
//         Self { trie }
//     }
// }
