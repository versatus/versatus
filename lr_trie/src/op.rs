pub type Byte = u8;
pub type Bytes = [Byte];

#[derive(Debug)]
#[non_exhaustive]
pub enum Operation<'a> {
    /// Add a single value serialized to bytes
    Add(&'a Bytes, &'a Bytes),

    /// Remove a value specified by the key from the trie
    Remove(&'a Bytes),

    /// Extend the state trie with the provided iterator over leaf values as byte slices.
    Extend(Vec<(&'a Bytes, &'a Bytes)>),
}
