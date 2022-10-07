pub type Byte = u8;
pub type Bytes = [Byte];

#[derive(Debug)]
#[non_exhaustive]
pub enum Operation {
    /// Add a single value serialized to bytes
    Add(Vec<u8>, Vec<u8>),

    /// Remove a value specified by the key from the trie
    Remove(Vec<u8>),

    /// Extend the state trie with the provided iterator over leaf values as
    /// byte slices.
    // Extend(Vec<(&'a Bytes, &'a Bytes)>),
    Extend(Vec<(Vec<u8>, Vec<u8>)>),
}
