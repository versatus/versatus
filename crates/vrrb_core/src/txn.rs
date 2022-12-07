#![allow(unused_imports, dead_code)]
use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use bytebuffer::ByteBuffer;
use primitives::types::PublicKey;
use secp256k1::ecdsa::Signature;
use secp256k1::{Message, Secp256k1};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sha256::digest;
use uuid::Uuid;
use primitives::types::PublicKeyBytes;

/// This module contains the basic structure of simple transaction
use crate::accountable::Accountable;
use crate::verifiable::Verifiable;

/// A simple custom error type
#[derive(thiserror::Error, Clone, Debug, Serialize, Deserialize)]
pub enum TxnError {
    #[error("invalid transaction: {0}")]
    InvalidTxn(String),
}

pub type TxNonce = u128;
pub type TxTimestamp = i64;
pub type TxAmount = u128;
pub type TxSignature = Vec<u8>;
pub type TxPayload = String;

// TODO: replace with a generic token struct
pub type TxToken = String;

/// The basic transation structure.
//TODO: Discuss the pieces of the Transaction structure that should stay and go
//TODO: Discuss how to best package this to minimize the size of it/compress it
//TODO: Change `validators` filed to `receipt` or `certificate` to put threshold
//signature of validators in.
#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Txn {
    // TODO: Make all fields private
    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: PublicKey,
    pub receiver_address: String,
    pub token: Option<TxToken>,
    pub amount: TxAmount,
    pub payload: Option<TxPayload>,
    pub signature: TxSignature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

#[derive(Debug, Clone)]
pub struct NewTxnArgs {
    pub sender_address: String,
    pub sender_public_key: PublicKey,
    pub receiver_address: String,
    pub token: Option<TxToken>,
    pub amount: TxAmount,
    pub payload: Option<TxPayload>,
    pub signature: TxSignature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

impl Txn {
    pub fn new(args: NewTxnArgs) -> Self {
        let timestamp = chrono::offset::Utc::now().timestamp();

        Self {
            timestamp,
            sender_address: args.sender_address,
            sender_public_key: args.sender_public_key,
            receiver_address: args.receiver_address,
            token: args.token,
            amount: args.amount,
            payload: args.payload,
            signature: args.signature,
            validators: args.validators,
            nonce: args.nonce,
        }
    }

    /// Produces a SHA 256 hash string of the transaction
    pub fn digest(&self) -> String {
        let encoded = self.encode();

        digest(encoded.as_slice())
    }

    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    #[deprecated(note = "use encode instead")]
    pub fn as_bytes(&self) -> Vec<u8> {
        let as_string = serde_json::to_string(self).unwrap();
        as_string.as_bytes().to_vec()
    }

    #[deprecated(note = "rely on the from trait implementation instead")]
    pub fn from_bytes(data: &[u8]) -> Txn {
        Self::from(data)
    }

    #[deprecated(note = "rely on the from trait implementation instead")]
    pub fn from_string(string: &str) -> Txn {
        Self::from(string)
    }

    pub fn is_null(&self) -> bool {
        self == &NULL_TXN
    }
}

pub const NULL_TXN: Txn = Txn {
    timestamp: 0,
    sender_address: String::new(),
    sender_public_key: vec![],
    receiver_address: String::new(),
    token: None,
    amount: 0,
    payload: None,
    signature: vec![],
    validators: None,
    nonce: 0,
};

impl From<String> for Txn {
    fn from(data: String) -> Self {
        data.parse().unwrap_or(NULL_TXN)
    }
}

impl From<Vec<u8>> for Txn {
    fn from(data: Vec<u8>) -> Self {
        serde_json::from_slice::<Txn>(&data).unwrap_or(NULL_TXN)
    }
}

impl From<&[u8]> for Txn {
    fn from(data: &[u8]) -> Self {
        serde_json::from_slice::<Txn>(data).unwrap_or(NULL_TXN)
    }
}

impl FromStr for Txn {
    type Err = TxnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Txn>(s)
            .map_err(|err| TxnError::InvalidTxn(format!("failed to parse &str into Txn: {err}")))
    }
}

impl From<&str> for Txn {
    fn from(data: &str) -> Self {
        data.parse().unwrap_or(NULL_TXN)
    }
}

impl fmt::Display for Txn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Txn(\n \
            timestamp: {},\n \
            sender_address: {:?},\n \
            sender_public_key: {:?},\n \
            receiver_address: {:?},\n \
            token: {:?},\n \
            amount: {},\n \
            signature: {:?}",
            self.timestamp,
            self.sender_address,
            hex::encode(self.sender_public_key.clone()),
            self.receiver_address,
            self.token,
            self.amount,
            self.signature,
        )
    }
}

impl Hash for Txn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
        self.sender_address.hash(state);
        self.sender_public_key.hash(state);
        self.receiver_address.hash(state);
        self.token.hash(state);
        self.amount.hash(state);
        self.payload.hash(state);
        self.signature.hash(state);
        self.nonce.hash(state);
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state);
        }
    }
}

impl PartialEq for Txn {
    fn eq(&self, other: &Self) -> bool {
        self.digest() == other.digest()
    }
}
