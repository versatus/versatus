pub type Byte = u8;
pub type Bytes = [Byte];
pub type Key = Vec<Byte>;
pub type TrieValue = Vec<Byte>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Operation {
    /// Add a single value serialized to bytes
    Add(Key, TrieValue),

    /// Update a single value serialized to bytes
    Update(Key, TrieValue),

    /// Remove a value specified by the key from the trie
    Remove(Key),
}
