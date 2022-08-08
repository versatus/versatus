use crate::{Byte, Bytes, Operation};
use left_right::{Absorb, ReadHandle, ReadHandleFactory, WriteHandle};
use rs_merkle::{Hasher, MerkleTree};
use std::fmt::Debug;

// TODO; impl Debug on MerkleTree
#[derive(Clone)]
pub struct InnerTrie<H: Hasher> {
    mt: MerkleTree<H>,
}

/// Base trie struct, meant to be used by LeftRightTrie through read and write handlers
impl<H: Hasher> InnerTrie<H> {
    /// Creates a new empty trie.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single leaf value serialized to bytes
    /// Example:
    /// ```
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///  use lr_trie::inner::InnerTrie;
    ///
    ///  let mut inner_trie = InnerTrie::<Sha256>::new();
    ///
    ///  inner_trie.add(String::from("hello world").as_bytes());
    ///
    ///  assert_eq!(inner_trie.len(), 1);
    /// ```
    ///
    pub fn add(&mut self, value: &Bytes) {
        let hashed = H::hash(value);

        self.mt.insert(hashed).commit();
    }

    /// Extends the state trie with the provided iterator over leaf values as byte slices.
    /// Example:
    /// ```
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///  use lr_trie::inner::InnerTrie;
    ///
    ///  let mut inner_trie = InnerTrie::<Sha256>::new();
    ///  
    ///  inner_trie.extend(
    ///      vec![
    ///          String::from("abcdefg").as_bytes(),
    ///          String::from("hijkl").as_bytes(),
    ///          String::from("mnopq").as_bytes(),
    ///      ]
    ///      .into_iter(),
    ///  );
    ///  
    ///  assert_eq!(inner_trie.len(), 3);
    /// ```
    ///
    // pub fn extend<'a, T>(&mut self, values: T)
    pub fn extend<'a, T>(&mut self, values: T)
    where
        T: Iterator<Item = &'a Bytes>,
        // T: Iterator<Item = H::Hash>,
    {
        let mut hashed_values = values.map(|val| H::hash(val)).collect::<Vec<H::Hash>>();

        self.mt.append(&mut hashed_values).commit();
    }

    /// Returns the trie's Merkle root.
    /// Example:
    /// ```
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///  use lr_trie::inner::InnerTrie;
    ///
    ///  let inner_trie_a = InnerTrie::<Sha256>::from(vec![
    ///        Sha256::hash("abcdefg".as_bytes()),
    ///        Sha256::hash("hijkl".as_bytes()),
    ///        Sha256::hash("mnopq".as_bytes()),
    ///  ]);
    ///
    ///  let inner_trie_b = InnerTrie::<Sha256>::from(vec![
    ///        Sha256::hash("abcdefg".as_bytes()),
    ///        Sha256::hash("hijkl".as_bytes()),
    ///        Sha256::hash("mnopq".as_bytes()),
    ///  ]);
    ///
    ///  assert_eq!(inner_trie_a.root(), inner_trie_b.root());
    /// ```
    ///
    pub fn root(&self) -> Option<H::Hash> {
        self.mt.root()
    }

    /// Returns the count of leaves in the state trie.
    /// Example:
    /// ```
    ///  use lr_trie::inner::InnerTrie;
    ///  use rs_merkle::algorithms::Sha256;
    ///  use rs_merkle::Hasher;
    ///
    ///  let inner_trie = InnerTrie::<Sha256>::from(vec![
    ///        Sha256::hash("abcdefg".as_bytes()),
    ///        Sha256::hash("hijkl".as_bytes()),
    ///        Sha256::hash("mnopq".as_bytes()),
    ///  ]);
    ///
    ///  assert_eq!(inner_trie.len(), 3);
    /// ```
    ///
    pub fn len(&self) -> usize {
        self.mt.leaves_len()
    }

    /// Returns true if there are no values in the trie.
    /// Example:
    /// ```
    ///  use lr_trie::inner::InnerTrie;
    ///  use rs_merkle::algorithms::Sha256;
    ///
    ///  let inner_trie = InnerTrie::<Sha256>::new();
    ///
    ///  assert_eq!(inner_trie.is_empty(), true);
    /// ```
    ///
    pub fn is_empty(&self) -> bool {
        self.mt.leaves_len() == 0
    }

    pub fn leaves(&self) -> Option<Vec<H::Hash>> {
        self.mt.leaves()
    }
}

impl<H: Hasher> PartialEq for InnerTrie<H> {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root()
    }
}

impl<H: Hasher> Default for InnerTrie<H> {
    fn default() -> Self {
        Self {
            mt: Default::default(),
        }
    }
}

impl<H: Hasher> Debug for InnerTrie<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("InnerTrie");
        debug.finish()
    }
}

impl<H: Hasher> From<Vec<H::Hash>> for InnerTrie<H> {
    fn from(values: Vec<H::Hash>) -> Self {
        let mt = MerkleTree::from_leaves(&values);
        Self { mt }
    }
}

impl<'a, H> Absorb<Operation<'a>> for InnerTrie<H>
where
    H: Hasher,
{
    fn absorb_first(&mut self, operation: &mut Operation<'a>, _other: &Self) {
        match operation {
            Operation::Add(data) => {
                self.add(data);
            }
            Operation::Extend(data) => {
                let data = data.iter().copied();

                self.extend(data);
            }
        }
    }

    fn sync_with(&mut self, first: &Self) {
        *self = first.clone();
    }
}

#[cfg(test)]
mod tests {
    use std::thread::{self, JoinHandle};

    use super::*;
    use rs_merkle::algorithms::Sha256;
    use rs_merkle::Hasher;

    #[test]
    fn new_creates_default_empty_trie() {
        let inner_trie = InnerTrie::<Sha256>::new();

        assert_eq!(inner_trie.root(), None);
        assert_eq!(inner_trie.len(), 0);
    }

    #[test]
    fn new_creates_trie_from_lrdb_values() {
        let inner_trie = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        let hash_bytes = [
            91, 42, 162, 88, 248, 119, 77, 41, 94, 6, 35, 62, 123, 36, 207, 69, 207, 94, 77, 139,
            158, 84, 143, 35, 127, 118, 132, 211, 125, 226, 23, 147,
        ];

        assert_eq!(inner_trie.len(), 3);
        assert_eq!(inner_trie.root(), Some(hash_bytes));
    }

    #[test]
    fn should_add_node_to_trie() {
        let mut inner_trie = InnerTrie::<Sha256>::new();

        assert_eq!(inner_trie.root(), None);
        assert_eq!(inner_trie.len(), 0);

        inner_trie.add(String::from("hello world").as_bytes());

        assert_ne!(inner_trie.root(), None);
        assert_eq!(inner_trie.len(), 1);
    }

    #[test]
    fn should_extend_trie_with_nodes() {
        let mut inner_trie = InnerTrie::<Sha256>::new();

        assert_eq!(inner_trie.root(), None);
        assert_eq!(inner_trie.len(), 0);

        inner_trie.extend(
            vec![
                String::from("abcdefg").as_bytes(),
                String::from("hijkl").as_bytes(),
                String::from("mnopq").as_bytes(),
            ]
            .into_iter(),
        );

        assert_ne!(inner_trie.root(), None);
        assert_eq!(inner_trie.len(), 3);
    }

    #[test]
    fn should_return_true_if_root_is_equal_to_other_trie_root() {
        let inner_trie_a = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        let inner_trie_b = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        assert_eq!(inner_trie_a, inner_trie_b);
    }

    #[test]
    fn should_return_false_if_root_is_not_equal_to_other_trie_root() {
        let inner_trie_a = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
        ]);

        let inner_trie_b = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
            Sha256::hash("rstuv".as_bytes()),
        ]);

        assert_ne!(inner_trie_a, inner_trie_b);
    }

    #[test]
    fn should_apply_op_add() {
        let mut inner_trie = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
            Sha256::hash("rstuv".as_bytes()),
        ]);

        let previous_inner_trie = inner_trie.clone();

        let new_entry = Sha256::hash("abcdefg".as_bytes());

        let mut op = Operation::Add(&new_entry);

        inner_trie.absorb_first(&mut op, &previous_inner_trie);

        assert_eq!(inner_trie.len(), 5);
    }

    #[test]
    fn should_apply_op_extend() {
        let mut inner_trie = InnerTrie::<Sha256>::from(vec![
            Sha256::hash("abcdefg".as_bytes()),
            Sha256::hash("hijkl".as_bytes()),
            Sha256::hash("mnopq".as_bytes()),
            Sha256::hash("rstuv".as_bytes()),
        ]);

        let previous_inner_trie = inner_trie.clone();

        let new_entries = vec!["abcdefg".as_bytes(), "hijkl".as_bytes()];

        let mut op = Operation::Extend(new_entries);

        inner_trie.absorb_first(&mut op, &previous_inner_trie);

        assert_eq!(inner_trie.len(), 6);
    }
}
