use keccak_hash::H256;

use crate::db::Database;
use crate::result::Result;

pub trait Trie<D: Database> {
    /// Returns the value for key stored in the trie.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Returns true if the key is present within the trie
    fn contains(&self, key: &[u8]) -> Result<bool>;

    /// Inserts value into trie and modifies it if it exists
    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Removes any existing value for key from the trie.
    fn remove(&mut self, key: &[u8]) -> Result<bool>;

    /// Returns the root hash of the trie. This is an expensive operation as it commits every node
    /// in the cache to the database to recalculate the root.
    fn root_hash(&mut self) -> Result<H256>;

    /// Prove constructs a merkle proof for key. The result contains all encoded nodes
    /// on the path to the value at key. The value itself is also included in the last
    /// node and can be retrieved by verifying the proof.
    ///
    /// If the trie does not contain a value for key, the returned proof contains all
    /// nodes of the longest existing prefix of the key (at least the root node), ending
    /// with the node that proves the absence of the key.
    // TODO refactor encode_raw() so that it doesn't need a &mut self
    // TODO (Daniel): refactor and potentially submit a patch upstream
    fn get_proof(&mut self, key: &[u8]) -> Result<Vec<Vec<u8>>>;

    /// Returns a value if key exists, None if key doesn't exist, Error if proof is wrong
    fn verify_proof(
        &self,
        root_hash: H256,
        key: &[u8],
        proof: Vec<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>>;
}
