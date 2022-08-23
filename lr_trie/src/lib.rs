/// This crate contains a left-right wrapped Merkle-Patricia Trie
/// heavily inspired by https://github.com/carver/eth-trie.rs which is a fork of https://github.com/citahub/cita-trie

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

pub mod db;
pub mod error;
pub mod inner;
mod inner_trie;
mod lr_trie;
pub(crate) mod nibbles;
pub(crate) mod node;

pub use crate::lr_trie::*;
