/// This crate contains a left-right wrapped, evmap-backed, Merkle-Patricia Trie
/// heavily inspired by https://github.com/carver/eth-trie.rs which is a fork of https://github.com/citahub/cita-trie
///
pub mod db;
pub mod error;
pub mod inner;
mod lr_trie;
pub mod result;
pub mod trie;

pub(crate) mod nibbles;
pub(crate) mod node;
pub mod op;

pub use crate::lr_trie::*;
pub use crate::op::*;
