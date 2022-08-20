#![allow(unused_imports, dead_code)]
/// This module contains the basic structure of simple transaction
use accountable::accountable::Accountable;
use bytebuffer::ByteBuffer;
use pool::pool::Pool;
use secp256k1::{Message, PublicKey, Secp256k1, Signature};
use serde::{Deserialize, Serialize};
use sha256::digest_bytes;
use state::state::NetworkState;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use verifiable::verifiable::Verifiable;

/// A simple custom error type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvalidTxnError {
    details: String,
}

/// The basic transation structure.
//TODO: Discuss the pieces of the Transaction structure that should stay and go
//TODO: Discuss how to best package this to minimize the size of it/compress it 
//TODO: Change `validators` filed to `receipt` or `certificate` to put threshold
//signature of validators in. 
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Txn {
    pub txn_id: String,
    pub txn_timestamp: u128,
    pub sender_address: String,
    pub sender_public_key: String,
    pub receiver_address: String,
    pub txn_token: Option<String>,
    pub txn_amount: u128,
    pub txn_payload: String,
    pub txn_signature: String,
    pub validators: HashMap<String, bool>,
    pub nonce: u128,
}

impl Txn {
    // TODO: convert to_message into a function of the verifiable trait,
    // all verifiable objects need to be able to be converted to a message.
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let as_string = serde_json::to_string(self).unwrap();
        as_string.as_bytes().iter().copied().collect()
    }

    pub fn from_bytes(data: &[u8]) -> Txn {
        serde_json::from_slice::<Txn>(data).unwrap()
    }

    pub fn from_string(string: &String) -> Txn {
        serde_json::from_str::<Txn>(string).unwrap()
    }

    pub fn get_field_names(&self) -> Vec<String> {
        vec![
            "txn_id".to_string(),
            "txn_timestamp".to_string(),
            "sender_address".to_string(),
            "sender_public_key".to_string(),
            "receiver_address".to_string(),
            "txn_token".to_string(),
            "txn_amount".to_string(),
            "txn_payload".to_string(),
            "txn_signature".to_string(),
            "txn_signature".to_string(),
            "validators".to_string(),
            "nonce".to_string(),
        ]
    }
}

impl Accountable for Txn {
    type Category = Option<String>;

    fn receivable(&self) -> String {
        self.receiver_address.clone()
    }

    fn payable(&self) -> Option<String> {
        Some(self.sender_address.clone())
    }
    fn get_amount(&self) -> u128 {
        self.txn_amount
    }
    fn get_category(&self) -> Option<Self::Category> {
        None
    }
}

impl Verifiable for Txn {
    type Item = Option<String>;
    type Dependencies = (NetworkState, Pool<String, Txn>);
    type Error = InvalidTxnError;

    fn verifiable(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn valid(
        &self,
        item: &Self::Item,
        dependencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl std::fmt::Display for InvalidTxnError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "InvalidTxn: {}", self.details)
    }
}

impl std::error::Error for InvalidTxnError {
    fn description(&self) -> &str {
        "Invalid Transaction Error"
    }
}

impl fmt::Display for Txn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Txn(\n \
            txn_id: {},\n \
            txn_timestamp: {},\n \
            sender_address: {},\n \
            sender_public_key: {},\n \
            receiver_address: {},\n \
            txn_token: {:?},\n \
            txn_amount: {},\n \
            txn_signature: {}",
            self.txn_id,
            self.txn_timestamp.to_string(),
            self.sender_address,
            self.sender_public_key,
            self.receiver_address,
            self.txn_token,
            self.txn_amount,
            self.txn_signature,
        )
    }
}
