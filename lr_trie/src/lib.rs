pub type Byte = u8;
pub type Bytes = [Byte];

#[derive(Debug)]
#[non_exhaustive]
pub enum Operation<'a> {
    /// Add a single leaf value serialized to bytes
    Add(&'a Bytes),

    /// Extend the state trie with the provided iterator over leaf values as byte slices.
    Extend(Vec<&'a Bytes>),
}

pub mod inner;
mod lr_trie;
mod trie;

pub use crate::lr_trie::*;
pub use crate::trie::*;
