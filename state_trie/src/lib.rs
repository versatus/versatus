use rs_merkle::{Hasher, MerkleTree};
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

// TODO; impl Debug on MerkleTree
// #[derive(Debug)]
pub struct StateTrie<H: Hasher> {
    mt: MerkleTree<H>,
}

impl<H: Hasher> StateTrie<H> {
    pub fn new() -> Self {
        Self {
            mt: MerkleTree::<H>::new(),
        }
    }

    pub fn from(db: Vec<u8>) -> Self {
        todo!();
    }

    pub fn add_node<'a, A>(&mut self, account: A)
    where
        A: Into<&'a [u8]>,
    {
        let hashed = H::hash(account.into());

        self.mt.insert(hashed);
        self.mt.commit();
    }

    pub fn verify(&mut self) {
        todo!();
    }

    pub fn nodes_len(&self) -> usize {
        self.mt.leaves_len()
    }
}

// TODO
// impl<H: Hasher> From<LeftRightDb> for StateTrie<H> {
//     fn from(db: T) -> Self {
//         // TODO: implement
//     }
// }

impl<T: Hasher> Debug for StateTrie<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: derive once MerkleTree impl Debug
        f.debug_struct("StateTrie").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_merkle::algorithms::Sha256;

    #[test]
    fn new_creates_default_empty_trie() {
        let state_trie = StateTrie::<Sha256>::new();
        assert_eq!(state_trie.nodes_len(), 0);
    }

    #[test]
    fn should_add_nodes_to_trie() {
        let state_trie = StateTrie::<Sha256>::new();
        // todo!();
    }
}
