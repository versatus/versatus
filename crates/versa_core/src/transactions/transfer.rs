use std::{
    cmp::{Ord, Ordering, PartialOrd},
    collections::HashMap,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    str::FromStr,
};

use primitives::{
    Address, ByteSlice, ByteVec, Digest as PrimitiveDigest, NodeIdx, PublicKey, RawSignature,
    SecretKey,
};
use secp256k1::{ecdsa::Signature, Message};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utils::hash_data;

use crate::transactions::transaction::Transaction;
use crate::transactions::{Token, TransactionDigest, TransactionKind, BASE_FEE};
use crate::{
    helpers::gen_hex_encoded_string,
    keypair::Keypair,
    serde_helpers::{
        decode_from_binary_byte_slice, decode_from_json_byte_slice, encode_to_binary,
        encode_to_json,
    },
};
/// This module contains the basic structure of simple transaction

/// A simple custom error type
#[derive(thiserror::Error, Clone, Debug, Serialize, Deserialize)]
pub enum TransferTransactionError {
    #[error("invalid transaction: {0}")]
    InvalidTransferTransaction(String),
}

pub fn generate_transfer_digest_vec(
    timestamp: TxTimestamp,
    sender_address: String,
    sender_public_key: PublicKey,
    receiver_address: String,
    token: Token,
    amount: TxAmount,
    nonce: TxNonce,
) -> ByteVec {
    let payload_string = format!(
        "{},{},{},{},{},{:?},{}",
        &timestamp, &sender_address, &sender_public_key, &receiver_address, &amount, &token, &nonce
    );

    let mut hasher = Sha256::new();
    hasher.update(payload_string);
    let hash = hasher.finalize();

    hash.to_vec()
}

pub type TxNonce = u128;
pub type TxTimestamp = i64;
pub type TxAmount = u128;
pub type TxSignature = Vec<u8>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Transfer {
    pub id: TransactionDigest,
    pub timestamp: TxTimestamp,
    pub sender_address: Address,
    pub sender_public_key: PublicKey,
    pub receiver_address: Address,
    pub token: Token,
    pub amount: TxAmount,
    pub signature: Signature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

#[derive(Clone, Default)]
pub struct TransferBuilder {
    timestamp: Option<TxTimestamp>,
    sender_address: Option<Address>,
    sender_public_key: Option<PublicKey>,
    receiver_address: Option<Address>,
    token: Option<Token>,
    amount: Option<TxAmount>,
    signature: Option<Signature>,
    validators: Option<HashMap<String, bool>>,
    nonce: Option<TxNonce>,
}

impl TransferBuilder {

    pub fn timestamp(mut self, timestamp: TxTimestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn sender_address(mut self, sender_address: Address) -> Self {
        self.sender_address = Some(sender_address);
        self
    }

    pub fn sender_public_key(mut self, sender_public_key: PublicKey) -> Self {
        self.sender_public_key = Some(sender_public_key);
        self
    }

    pub fn receiver_address(mut self, receiver_address: Address) -> Self {
        self.receiver_address = Some(receiver_address);
        self
    }

    pub fn token(mut self, token: Token) -> Self {
        self.token = Some(token);
        self
    }

    pub fn amount(mut self, amount: TxAmount) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn signature(mut self, signature: Signature) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn validators(mut self, validators: HashMap<String, bool>) -> Self {
        self.validators = Some(validators);
        self
    }

    pub fn nonce(mut self, nonce: TxNonce) -> Self {
        self.nonce = Some(nonce);
        self
    }

    pub fn build(self) -> Result<Transfer, &'static str> {
        let id = generate_transfer_digest_vec(
            self.timestamp.ok_or("timestamp is missing")?,
            self.sender_address.clone().ok_or("sender_address is missing")?.to_string(),
            self.sender_public_key.ok_or("sender_public_key is missing")?,
            self.receiver_address.clone().ok_or("receiver_address is missing")?.to_string(),
            self.token.clone().unwrap_or_default(),
            self.amount.ok_or("amount is missing")?,
            self.nonce.ok_or("nonce is missing")?,
        );

        Ok(Transfer {
            id: TransactionDigest::from(id),
            timestamp: self.timestamp.unwrap(),
            sender_address: self.sender_address.unwrap(),
            sender_public_key: self.sender_public_key.unwrap(),
            receiver_address: self.receiver_address.unwrap(),
            token: self.token.unwrap_or_default(),
            amount: self.amount.unwrap(),
            signature: self.signature.ok_or("signature is missing")?,
            validators: self.validators,
            nonce: self.nonce.unwrap(),
        })
    }

    pub fn build_kind(self) -> Result<TransactionKind, &'static str> {
        let transfer = self.build().expect("failed to build transfer");

        Ok(TransactionKind::Transfer(transfer))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTransferArgs {
    pub timestamp: TxTimestamp,
    pub sender_address: Address,
    pub sender_public_key: PublicKey,
    pub receiver_address: Address,
    pub token: Option<Token>,
    pub amount: TxAmount,
    pub signature: Signature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

impl Default for Transfer {
    fn default() -> Self {
        Transfer::null_txn()
    }
}

impl Transfer {
    pub fn builder() -> TransferBuilder {
        TransferBuilder::default()
    }
    pub fn new(args: NewTransferArgs) -> Self {
        let token = args.token.clone().unwrap_or_default();

        let digest_vec = generate_transfer_digest_vec(
            args.timestamp.clone(),
            args.sender_address.to_string(),
            args.sender_public_key,
            args.receiver_address.to_string(),
            token.clone(),
            args.amount.clone(),
            args.nonce.clone(),
        );

        let digest = TransactionDigest::from(digest_vec);

        Self {
            id: digest,
            // TODO: change time unit from seconds to millis
            timestamp: args.timestamp,
            sender_address: args.sender_address,
            sender_public_key: args.sender_public_key,
            receiver_address: args.receiver_address,
            token,
            amount: args.amount,
            signature: args.signature,
            validators: args.validators,
            nonce: args.nonce,
        }
    }

    pub fn null_txn() -> Transfer {
        let timestamp = chrono::Utc::now().timestamp();
        let kp = Keypair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key);

        let digest_vec = generate_transfer_digest_vec(
            timestamp,
            address.to_string(),
            public_key,
            address.to_string(),
            Token::default(),
            0,
            0,
        );

        let digest = TransactionDigest::from(digest_vec);

        let payload = utils::hash_data!(
            timestamp.to_string(),
            address.to_string(),
            public_key.to_string(),
            address.to_string(),
            Token::default().to_string(),
            0.to_string(),
            0.to_string()
        );

        type H = secp256k1::hashes::sha256::Hash;
        let msg = Message::from_hashed_data::<H>(&payload[..]);
        let signature = kp.miner_kp.0.sign_ecdsa(msg);

        Self {
            id: digest,
            // TODO: change time unit from seconds to millis
            timestamp,
            sender_address: address.clone(),
            sender_public_key: kp.miner_kp.1,
            receiver_address: address,
            token: Token::default(),
            amount: 0,
            signature,
            validators: None,
            nonce: 0,
        }
    }

    pub fn build_payload_digest(&self) -> TransactionDigest {
        let digest = generate_transfer_digest_vec(
            self.timestamp(),
            self.sender_address().to_string(),
            self.sender_public_key(),
            self.receiver_address().to_string(),
            self.token(),
            self.amount(),
            self.nonce(),
        );

        digest.into()
    }

    #[deprecated]
    pub fn txn_id(&self) -> String {
        self.id().to_string()
    }

    /// Serializes the transation into a byte array
    pub fn encode(&self) -> Vec<u8> {
        encode_to_binary(self).unwrap_or_default()
    }

    /// Encodes the transaction into a JSON-serialized byte vector
    pub fn encode_to_json(&self) -> Vec<u8> {
        encode_to_json(self).unwrap_or_default()
    }

    pub fn is_null(&self) -> bool {
        self == &Transfer::null_txn()
    }

    pub fn generate_txn_digest_vec(&self) -> ByteVec {
        generate_transfer_digest_vec(
            self.timestamp(),
            self.sender_address().to_string(),
            self.sender_public_key(),
            self.receiver_address().to_string(),
            self.token(),
            self.amount(),
            self.nonce(),
        )
    }

    fn from_byte_slice(data: ByteSlice) -> Self {
        if let Ok(txn) = decode_from_json_byte_slice::<Self>(data) {
            return txn;
        }

        if let Ok(txn) = decode_from_binary_byte_slice::<Self>(data) {
            return txn;
        }

        Transfer::null_txn()
    }

    fn from_string(data: &str) -> Transfer {
        if let Ok(txn) = Transfer::from_str(data) {
            return txn;
        }

        Transfer::null_txn()
    }
}

impl Transaction for Transfer {
    /// Produces a SHA 256 hash slice of bytes from the transaction
    fn id(&self) -> TransactionDigest {
        self.id.clone()
    }
    fn timestamp(&self) -> TxTimestamp {
        self.timestamp
    }

    fn sender_address(&self) -> Address {
        self.sender_address.clone()
    }

    fn sender_public_key(&self) -> PublicKey {
        self.sender_public_key
    }

    fn receiver_address(&self) -> Address {
        self.receiver_address.clone()
    }

    fn token(&self) -> Token {
        self.token.clone()
    }

    fn amount(&self) -> TxAmount {
        self.amount
    }

    fn signature(&self) -> Signature {
        self.signature
    }

    fn validators(&self) -> Option<HashMap<String, bool>> {
        self.validators.clone()
    }

    fn nonce(&self) -> TxNonce {
        self.nonce
    }

    fn fee(&self) -> u128 {
        BASE_FEE
    }

    fn validator_fee_share(&self) -> u128 {
        BASE_FEE / 2u128
    }

    fn proposer_fee_share(&self) -> u128 {
        BASE_FEE / 2u128
    }

    fn build_payload(&self) -> String {
        format!(
            "{:x}",
            hash_data!(
                self.sender_address.clone(),
                self.sender_public_key.clone(),
                self.receiver_address.clone(),
                self.token.clone(),
                self.amount.clone(),
                self.nonce.clone()
            )
        )
    }

    fn digest(&self) -> TransactionDigest {
        self.id()
    }

    fn sign(&mut self, sk: &SecretKey) {
        // TODO: refactor signing out the txn structure definition
        // TODO: return Result<(), SignatureError>
        let mut hasher = sha2::Sha256::new();
        hasher.update(self.build_payload().as_bytes());
        let result = hasher.finalize().to_vec();
        let message = Message::from_slice(&result);
        if let Ok(msg) = message {
            let sig = sk.sign_ecdsa(msg);
            self.signature = sig;
        }
    }
}

impl FromStr for Transfer {
    type Err = TransferTransactionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Transfer>(s)
            .map_err(|err| TransferTransactionError::InvalidTransferTransaction(format!("failed to parse &str into Txn: {err}")))
    }
}

impl fmt::Display for Transfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let txn_ser = serde_json::to_string_pretty(self).unwrap_or_default();

        write!(f, "{}", txn_ser)
    }
}

impl From<String> for Transfer {
    fn from(data: String) -> Self {
        Self::from(data.as_str())
    }
}

impl From<Vec<u8>> for Transfer {
    fn from(data: Vec<u8>) -> Self {
        Transfer::from_byte_slice(&data)
    }
}

impl From<&[u8]> for Transfer {
    fn from(data: &[u8]) -> Self {
        Transfer::from_byte_slice(data)
    }
}

impl From<&str> for Transfer {
    fn from(data: &str) -> Self {
        Self::from_string(data)
    }
}

impl From<Transfer> for TransactionDigest {
    fn from(txn: Transfer) -> Self {
        txn.id()
    }
}

impl Hash for Transfer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
        self.sender_address.hash(state);
        self.sender_public_key.hash(state);
        self.receiver_address.hash(state);
        self.token.hash(state);
        self.amount.hash(state);
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

impl PartialEq for Transfer {
    fn eq(&self, other: &Self) -> bool {
        self.generate_txn_digest_vec() == other.generate_txn_digest_vec()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTxnArgsError(String);

impl FromStr for NewTransferArgs {
    type Err = ParseTxnArgsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(|err| ParseTxnArgsError(err.to_string()))
    }
}
