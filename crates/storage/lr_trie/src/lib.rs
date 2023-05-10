/// This crate contains a left-right wrapped, evmap-backed, Merkle-Patricia Trie
/// heavily inspired by https://github.com/carver/eth-trie.rs which is a fork of https://github.com/citahub/cita-trie
pub use keccak_hash::H256;

mod inner_absorb;
mod inner_wrapper;
pub mod op;
mod result;
mod trie;

pub use crate::{inner_absorb::*, inner_wrapper::*, op::*, result::*, trie::*};
