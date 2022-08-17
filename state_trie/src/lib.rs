use lr_trie::{Bytes, LeftRightTrie};
use rs_merkle::Hasher;
use std::fmt::Debug;

// TODO; impl Debug on MerkleTree
pub struct StateTrie<'a, H: Hasher> {
    trie: LeftRightTrie<'a, H>,
}

impl<'a, H: Hasher> StateTrie<'a, H> {
    /// Creates a new empty state trie.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single leaf value serialized to bytes
    /// Example:
    /// ```
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///  use state_trie::StateTrie;
    ///
    ///  let mut state_trie = StateTrie::<Sha256>::new();
    ///
    ///  state_trie.add("hello world".as_bytes());
    ///
    ///  assert_eq!(state_trie.len(), 1);
    /// ```
    ///
    pub fn add(&mut self, value: &'a Bytes) {
        self.trie.add(value);
    }

    /// Extends the state trie with the provided iterator over leaf values as bytes.
    /// Example:
    /// ```
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///  use state_trie::StateTrie;
    ///
    ///  let mut state_trie = StateTrie::<Sha256>::new();
    ///  
    ///  state_trie.extend(
    ///      vec![
    ///          "abcdefg".as_bytes(),
    ///          "hijkl".as_bytes(),
    ///          "mnopq".as_bytes(),
    ///      ]
    ///  );
    ///  
    ///  assert_eq!(state_trie.len(), 3);
    /// ```
    ///
    pub fn extend(&mut self, values: Vec<&'a Bytes>) {
        self.trie.extend(values);
    }

    /// Returns the trie's Merkle root.
    /// Example:
    /// ```
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///  use state_trie::StateTrie;
    ///
    ///  let state_trie_a = StateTrie::<Sha256>::from(vec![
    ///        "abcdefg".as_bytes(),
    ///        "hijkl".as_bytes(),
    ///        "mnopq".as_bytes(),
    ///  ].into_iter());
    ///
    ///  let state_trie_b = StateTrie::<Sha256>::from(vec![
    ///        "abcdefg".as_bytes(),
    ///        "hijkl".as_bytes(),
    ///        "mnopq".as_bytes(),
    ///  ].into_iter());
    ///
    ///  assert_eq!(state_trie_a.root(), state_trie_b.root());
    /// ```
    ///
    pub fn root(&self) -> Option<H::Hash> {
        self.trie.root()
    }

    /// Returns the count of leaves in the state trie.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///
    ///  let state_trie = StateTrie::<Sha256>::from(vec![
    ///        "abcdefg".as_bytes(),
    ///        "hijkl".as_bytes(),
    ///        "mnopq".as_bytes(),
    ///  ].into_iter());
    ///
    ///  assert_eq!(state_trie.len(), 3);
    /// ```
    ///
    pub fn len(&self) -> usize {
        self.trie.len()
    }

    /// Returns true if there are no values in the trie.
    /// Example:
    /// ```
    ///  use state_trie::StateTrie;
    ///  use rs_merkle::algorithms::Sha256;
    ///
    ///  let state_trie = StateTrie::<Sha256>::new();
    ///
    ///  assert_eq!(state_trie.is_empty(), true);
    /// ```
    ///
    pub fn is_empty(&self) -> bool {
        self.trie.len() == 0
    }
}

impl<'a, H: Hasher> PartialEq for StateTrie<'a, H> {
    fn eq(&self, other: &Self) -> bool {
        self.trie == other.trie
    }
}

impl<'a, H: Hasher> Default for StateTrie<'a, H> {
    fn default() -> Self {
        Self {
            trie: Default::default(),
        }
    }
}

impl<'a, H: Hasher> Debug for StateTrie<'a, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: derive once MerkleTree impl Debug
        f.debug_struct("StateTrie").finish()
    }
}

impl<'a, H, E> From<E> for StateTrie<'a, H>
where
    H: Hasher,
    E: Iterator<Item = &'a Bytes>,
{
    fn from(values: E) -> Self {
        let trie = LeftRightTrie::from(values);
        Self { trie }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_merkle::algorithms::Sha256;

    #[test]
    fn new_creates_default_empty_trie() {
        let state_trie = StateTrie::<Sha256>::new();

        assert_eq!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 0);
    }

    #[test]
    fn new_creates_trie_from_lrdb_values() {
        let entries = vec!["abcdefg".as_bytes(), "hijkl".as_bytes(), "mnopq".as_bytes()];

        let state_trie = StateTrie::<Sha256>::from(entries.into_iter());

        let hash_bytes = [
            91, 42, 162, 88, 248, 119, 77, 41, 94, 6, 35, 62, 123, 36, 207, 69, 207, 94, 77, 139,
            158, 84, 143, 35, 127, 118, 132, 211, 125, 226, 23, 147,
        ];

        assert_eq!(state_trie.len(), 3);
        assert_eq!(state_trie.root(), Some(hash_bytes));
    }

    #[test]
    fn should_add_node_to_trie() {
        let mut state_trie = StateTrie::<Sha256>::new();

        assert_eq!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 0);

        let val = "hello world".as_bytes();
        state_trie.add(val);

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 1);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let mut state_trie = StateTrie::<Sha256>::new();

        assert_eq!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 0);

        let val_1 = "abcdefg".as_bytes();
        let val_2 = "hijkl".as_bytes();
        let val_3 = "mnopq".as_bytes();

        state_trie.extend(vec![val_1, val_2, val_3]);

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let state_trie_a = StateTrie::<Sha256>::from(
            vec!["abcdefg".as_bytes(), "hijkl".as_bytes(), "mnopq".as_bytes()].into_iter(),
        );

        let state_trie_b = StateTrie::<Sha256>::from(
            vec!["abcdefg".as_bytes(), "hijkl".as_bytes(), "mnopq".as_bytes()].into_iter(),
        );

        assert_eq!(state_trie_a, state_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let state_trie_a = StateTrie::<Sha256>::from(
            vec!["abcdefg".as_bytes(), "hijkl".as_bytes(), "mnopq".as_bytes()].into_iter(),
        );

        let state_trie_b = StateTrie::<Sha256>::from(
            vec![
                "abcdefg".as_bytes(),
                "hijkl".as_bytes(),
                "mnopq".as_bytes(),
                "rstuv".as_bytes(),
            ]
            .into_iter(),
        );

        assert_ne!(state_trie_a, state_trie_b);
    }
}
