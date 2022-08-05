use rs_merkle::{Hasher, MerkleTree};
use std::fmt::{Debug, Display};

// TODO; impl Debug on MerkleTree
// #[derive(Debug)]
pub struct StateTrie<H: Hasher> {
    mt: MerkleTree<H>,
}

impl<H: Hasher> StateTrie<H> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single leaf value serialized to bytes
    pub fn add(&mut self, account: &[u8]) {
        let hashed = H::hash(account);

        self.mt.insert(hashed).commit();
    }

    /// Extends the StateTrie with the provided iterator over leaf values as bytes.
    pub fn extend<'a, T>(&mut self, accounts: T)
    where
        T: Iterator<Item = &'a [u8]>,
    {
        let mut hashed_values = accounts.map(|acc| H::hash(acc)).collect::<Vec<H::Hash>>();

        self.mt.append(&mut hashed_values).commit();
    }

    pub fn root(&self) -> Option<H::Hash> {
        self.mt.root()
    }

    pub fn nodes_len(&self) -> usize {
        self.mt.leaves_len()
    }
}

impl<H: Hasher> PartialEq for StateTrie<H> {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root()
    }
}

impl<H: Hasher> Default for StateTrie<H> {
    fn default() -> Self {
        Self {
            mt: Default::default(),
        }
    }
}

impl<H: Hasher> Debug for StateTrie<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: derive once MerkleTree impl Debug
        f.debug_struct("StateTrie")
            // .field("mt", self.mt.leaves())
            .finish()
    }
}

impl<H: Hasher> From<Vec<H::Hash>> for StateTrie<H> {
    fn from(values: Vec<H::Hash>) -> Self {
        let mt = MerkleTree::from_leaves(&values);
        Self { mt }
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
        assert_eq!(state_trie.nodes_len(), 0);
    }

    #[test]
    fn new_creates_trie_from_lrdb_values() {
        let state_trie = StateTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        let hash_bytes = [
            91, 42, 162, 88, 248, 119, 77, 41, 94, 6, 35, 62, 123, 36, 207, 69, 207, 94, 77, 139,
            158, 84, 143, 35, 127, 118, 132, 211, 125, 226, 23, 147,
        ];

        assert_eq!(state_trie.nodes_len(), 3);
        assert_eq!(state_trie.root(), Some(hash_bytes));
    }

    #[test]
    fn should_add_node_to_trie() {
        let mut state_trie = StateTrie::<Sha256>::new();

        assert_eq!(state_trie.root(), None);
        assert_eq!(state_trie.nodes_len(), 0);

        state_trie.add(String::from("hello world").as_bytes());

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.nodes_len(), 1);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let mut state_trie = StateTrie::<Sha256>::new();

        assert_eq!(state_trie.root(), None);
        assert_eq!(state_trie.nodes_len(), 0);

        state_trie.extend(
            vec![
                String::from("abcdefg").as_bytes(),
                String::from("hijkl").as_bytes(),
                String::from("mnopq").as_bytes(),
            ]
            .into_iter(),
        );

        assert_ne!(state_trie.root(), None);
        assert_eq!(state_trie.nodes_len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let state_trie_a = StateTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        let state_trie_b = StateTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        assert_eq!(state_trie_a, state_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let state_trie_a = StateTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        let state_trie_b = StateTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
            Sha256::hash("rstuv".as_bytes()),
        ]);

        assert_ne!(state_trie_a, state_trie_b);
    }
}
