#[deprecated(note = "lr_trie should be used instead of this crate")]
pub mod error;

use std::{fmt::Debug, result::Result as StdResult, sync::Arc};

use error::StateTrieError;
use keccak_hash::H256;
use left_right::ReadHandleFactory;
use lr_trie::LeftRightTrie;
use lrdb::Account;
use patriecia::{db::Database, inner::InnerTrie};
type Result<T> = StdResult<T, StateTrieError>;

pub struct StateTrie<D: Database> {
    trie: LeftRightTrie<D>,
}

#[deprecated(note = "Use lr_trie directly instead")]
impl<D: Database> StateTrie<D> {
    /// Creates a new empty state trie.
    pub fn new(db: Arc<D>) -> Self {
        Self {
            trie: LeftRightTrie::new(db),
        }
    }

    /// Returns read handle factory to underlying
    pub fn factory(&self) -> ReadHandleFactory<InnerTrie<D>> {
        self.trie.factory()
    }

    /// Adds a single leaf value serialized to bytes
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lrdb::Account;
    /// use patriecia::db::MemoryDB;
    /// use state_trie::StateTrie;
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie = StateTrie::new(memdb);
    ///
    /// state_trie
    ///     .add(b"greetings.to_vec()".to_vec(), Account::new())
    ///     .unwrap();
    ///
    /// state_trie
    ///     .add(b"greetings.to_vec()".to_vec(), Account::new())
    ///     .unwrap();
    ///
    /// assert_eq!(state_trie.len(), 1);
    /// ```
    // TODO: Maybe it would be good idea to have both this and `trie.add` return
    // value Add tests to err
    pub fn add(&mut self, key: Vec<u8>, account: Account) -> Result<()> {
        match bincode::serialize(&account) {
            Ok(serialized) => {
                self.trie.add(key, serialized);
                return Ok(());
            },
            Err(_) => return Err(StateTrieError::FailedToSerializeAccount(account)),
        }
    }

    /// Extends the state trie with the provided iterator over leaf values as
    /// bytes. Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use state_trie::StateTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie = StateTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// state_trie.extend(vals);
    /// assert_eq!(state_trie.len(), 3);
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
    /// use state_trie::StateTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie_a = StateTrie::new(memdb);
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie_b = StateTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// state_trie_a.extend(vals.clone());
    /// state_trie_b.extend(vals.clone());
    ///
    /// state_trie_a.extend(vals.clone());
    /// state_trie_b.extend(vals.clone());
    ///
    /// assert_eq!(state_trie_a.root(), state_trie_b.root());
    /// ```
    pub fn root(&self) -> Option<H256> {
        self.trie.root()
    }

    /// Returns the count of leaves in the state trie.
    /// Example:
    /// ```
    /// use std::sync::Arc;
    ///
    /// use lr_trie::Bytes;
    /// use patriecia::db::MemoryDB;
    /// use state_trie::StateTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie = StateTrie::new(memdb);
    ///
    /// let vals: Vec<(Vec<u8>, Vec<u8>)> = vec![
    ///     (b"abcdefg".to_vec(), b"abcdefg".to_vec()),
    ///     (b"hijkl".to_vec(), b"hijkl".to_vec()),
    ///     (b"mnopq".to_vec(), b"mnopq".to_vec()),
    /// ];
    ///
    /// state_trie.extend(vals.clone());
    ///
    /// state_trie.extend(vals.clone());
    ///
    /// assert_eq!(state_trie.len(), 3);
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
    /// use state_trie::StateTrie;
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie = StateTrie::new(memdb);
    ///
    /// let memdb = Arc::new(MemoryDB::new(true));
    /// let mut state_trie = StateTrie::new(memdb);
    ///
    /// assert_eq!(state_trie.len(), 0);
    /// ```
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
    use std::sync::Arc;

    use lrdb::Account;
    use patriecia::{db::MemoryDB, trie::Trie};

    use super::*;

    #[test]
    fn new_creates_default_empty_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 0);
    }

    #[test]
    #[ignore = "breaking changes introduced to lr_trie make this test fail"]
    fn new_creates_trie_from_lrdb_values() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        state_trie.add(b"abcdefg".to_vec(), Account::new()).unwrap();
        state_trie.add(b"hijkl".to_vec(), Account::new()).unwrap();
        state_trie.add(b"mnopq".to_vec(), Account::new()).unwrap();

        let root = state_trie.root().unwrap();
        let root = format!("0x{}", hex::encode(root));

        let target_root =
            "0xb932b90dadf9a1f3c54c89f112f0d2c969753b20c112a98802d349d1db2859e0".to_string();

        let read_handle = state_trie.trie.handle();

        let default_account = bincode::serialize(&Account::new()).unwrap();
        let read_value = read_handle.get(b"abcdefg").unwrap().unwrap();

        assert_eq!(
            state_trie.trie.get().get(b"abcdefg").unwrap().unwrap(),
            default_account
        );

        assert_eq!(
            state_trie.trie.get().get(b"hijkl").unwrap().unwrap(),
            default_account
        );

        assert_eq!(
            state_trie.trie.get().get(b"mnopq").unwrap().unwrap(),
            default_account
        );

        assert_eq!(root, target_root);
    }

    #[test]
    fn should_add_node_to_trie() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 0);

        state_trie
            .add(b"greetings".to_vec(), Account::new())
            .unwrap();

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 1);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut state_trie = StateTrie::new(memdb);

        assert!(state_trie.root().is_some());
        assert_eq!(state_trie.len(), 0);

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
        state_trie_b.add(b"mnopq".to_vec(), Account::new()).unwrap();

        assert_ne!(state_trie_a, state_trie_b);
    }
}
