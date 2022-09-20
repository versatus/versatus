/// This crate contains a left-right wrapped, evmap-backed, Merkle-Patricia Trie
/// heavily inspired by https://github.com/carver/eth-trie.rs which is a fork of https://github.com/citahub/cita-trie
mod lr_trie;
pub mod op;

pub use keccak_hash::H256;

pub use crate::{lr_trie::*, op::*};
