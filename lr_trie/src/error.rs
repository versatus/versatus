use thiserror::Error;

use keccak_hash::H256;
use rlp::DecoderError;

use crate::nibbles::Nibbles;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum TrieError {
    #[error("invalid data")]
    InvalidData,

    #[error("invalid proof")]
    InvalidProof,

    #[error("missing node {node_hash:?}, root: {root_hash:?}")]
    MissingTrieNode {
        node_hash: H256,
        traversed: Option<Nibbles>,
        root_hash: Option<H256>,
        err_key: Option<Vec<u8>>,
    },

    #[error("database error: {0}")]
    Database(String),

    #[error("decoder error: {0}")]
    Decoder(#[from] DecoderError),
}

#[derive(Error, Debug)]
pub enum MemDBError {}
