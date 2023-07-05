/// This crate contains a left-right wrapped, evmap-backed, Merkle-Patricia Trie
/// heavily inspired by https://github.com/carver/eth-trie.rs which is a fork of https://github.com/citahub/cita-trie
pub use patriecia::H256;

mod inner;
mod inner_wrapper;
pub mod op;
mod result;
mod trie;

pub use crate::{inner::*, inner_wrapper::*, op::*, result::*, trie::*};
