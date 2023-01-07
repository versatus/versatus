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
use primitives::types::{PublicKey, SerializedPublicKey};
use secp256k1::{ecdsa::Signature, Message, Secp256k1};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sha256::digest;
use utils::{create_payload, hash_data, timestamp};
use uuid::Uuid;

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

//TODO: Replace with `secp256k1::Message` struct or guarantee
//that it is a stringified version of `secp256k1::Message`
pub type TxPayload = String;

// TODO: replace with a generic token struct
pub type TxToken = String;

/// The basic transation structure.
//TODO: Discuss the pieces of the Transaction structure that should stay and go
//TODO: Discuss how to best package this to minimize the size of it/compress it
//TODO: Change `validators` filed to `receipt` or `certificate` to put threshold
//signature of validators in.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq)]
pub struct Txn {
    // TODO: Make all fields private
    #[deprecated(note = "replaced by txn hash")]
    pub txn_id: Uuid,

    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: SerializedPublicKey,
    pub receiver_address: String,
    pub token: Option<TxToken>,
    pub amount: TxAmount,
    pub payload: Option<TxPayload>,
    pub signature: Option<TxSignature>,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

#[derive(Debug, Clone, Default)]
pub struct NewTxnArgs {
    pub sender_address: String,
    pub sender_public_key: SerializedPublicKey,
    pub receiver_address: String,
    pub token: Option<TxToken>,
    pub amount: TxAmount,
    pub payload: Option<TxPayload>,
    pub signature: Option<TxSignature>,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

impl Txn {
    pub fn new(args: NewTxnArgs) -> Self {
        // TODO: change time unit from seconds to millis
        let timestamp = timestamp!();

        Self {
            txn_id: Uuid::new_v4(),
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

    /// Encodes the transaction into a JSON-serialized byte vector
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

    pub fn amount(&self) -> TxAmount {
        self.amount
    }

    /// Alias for amount()
    pub fn get_amount(&self) -> TxAmount {
        self.amount()
    }

    pub fn token(&self) -> Option<TxToken> {
        self.token.clone()
    }

    pub fn set_token(&mut self, token: TxToken) {
        self.token = Some(token);
    }

    pub fn set_amount(&mut self, amount: u128) {
        self.amount = amount;
    }

    pub fn validators(&self) -> HashMap<String, bool> {
        self.validators.clone().unwrap_or_default()
    }

    pub fn txn_id(&self) -> String {
        self.txn_id.to_string()
    }

    pub fn payload(&self) -> String {
        self.payload.clone().unwrap_or_default()
    }

    pub fn build_payload(&mut self) {
        let payload = hash_data!(
            self.sender_address.clone(),
            self.sender_public_key.clone(),
            self.receiver_address.clone(),
            self.token.clone(),
            self.amount.clone(),
            self.nonce.clone()
        );

        self.payload = Some(payload);
    }

    pub fn sign(&mut self, sk: &secp256k1::SecretKey) {
        if let Some(payload) = self.payload.clone() {
            let message = Message::from_slice(payload.as_bytes());
            match message {
                Ok(msg) => {
                    let sig = sk.sign_ecdsa(msg);
                    self.signature = Some(sig.to_string().as_bytes().to_vec());
                },
                _ => { /*TODO return Result<(), SignatureError>*/ },
            }
        } else {
            self.build_payload();
            self.sign(&sk);
        }
    }
}

pub const NULL_TXN: Txn = Txn {
    txn_id: Uuid::nil(),
    timestamp: 0,
    sender_address: String::new(),
    sender_public_key: vec![],
    receiver_address: String::new(),
    token: None,
    amount: 0,
    payload: None,
    signature: None,
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
            self.sender_public_key,
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

// NOTE: temporary impl
// TODO: remove later
impl Accountable for Txn {
    type Category = ();

    fn receivable(&self) -> String {
        todo!()
    }

    fn payable(&self) -> Option<String> {
        todo!()
    }

    fn get_amount(&self) -> u128 {
        todo!()
    }

    fn get_category(&self) -> Option<Self::Category> {
        todo!()
    }
}

// NOTE: temporary impl
// TODO: remove later
impl Verifiable for Txn {
    type Dependencies = ();
    type Error = TxnError;
    type Item = Txn;

    fn verifiable(&self) -> bool {
        true
    }

    fn valid(
        &self,
        item: &Self::Item,
        debendencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
