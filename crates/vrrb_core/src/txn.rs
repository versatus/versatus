// TODO: Refactor and remove use of deprecated methods
#![allow(deprecated)]

use std::{
    cmp::{Ord, Ordering, PartialOrd},
    collections::HashMap,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    str::FromStr,
};

use primitives::{
    Address,
    ByteSlice,
    ByteVec,
    Digest as PrimitiveDigest,
    NodeIdx,
    PublicKey,
    RawSignature,
    SecretKey,
    DIGEST_LENGTH,
};
use secp256k1::{ecdsa::Signature, Message};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utils::hash_data;

use crate::{
    helpers::gen_hex_encoded_string,
    keypair::Keypair,
    serde_helpers::{
        decode_from_binary_byte_slice,
        decode_from_json_byte_slice,
        encode_to_binary,
        encode_to_json,
    },
};

pub const BASE_FEE: u128 = 0x2D79883D2000;
/// This module contains the basic structure of simple transaction

/// A simple custom error type
#[derive(thiserror::Error, Clone, Debug, Serialize, Deserialize)]
pub enum TxnError {
    #[error("invalid transaction: {0}")]
    InvalidTxn(String),
}

pub fn generate_txn_digest_vec(
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

// TODO: Replace with `secp256k1::Message` struct or guarantee
// that it is a stringified version of `secp256k1::Message`
pub type TxPayload = String;

// TODO: replace with a generic token struct
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token(name: {}, symbol: {}, decimals: {})",
            self.name, self.symbol, self.decimals
        )
    }
}

impl FromStr for Token {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let token: Token = serde_json::from_str(s)?;
        Ok(token)
    }
}

impl Default for Token {
    fn default() -> Self {
        Self {
            name: "VRRB".to_string(),
            symbol: "VRRB".to_string(),
            decimals: 18,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Hash, Clone, PartialEq, Eq)]
pub struct VoteReceipt {
    /// The identity of the voter.
    pub farmer_id: Vec<u8>,
    pub farmer_node_id: NodeIdx,
    /// Partial Signature
    pub signature: RawSignature,
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuorumCertifiedTxn {
    sender_farmer_id: Vec<u8>,
    /// All valid vote receipts
    votes: Vec<VoteReceipt>,
    txn: Txn,
    /// Threshold Signature
    signature: RawSignature,
    pub is_txn_valid: bool,
}

impl QuorumCertifiedTxn {
    pub fn new(
        sender_farmer_id: Vec<u8>,
        votes: Vec<VoteReceipt>,
        txn: Txn,
        signature: RawSignature,
        is_txn_valid: bool,
    ) -> QuorumCertifiedTxn {
        QuorumCertifiedTxn {
            sender_farmer_id,
            votes,
            txn,
            signature,
            is_txn_valid,
        }
    }

    pub fn txn(&self) -> Txn {
        self.txn.clone()
    }

    pub fn get_fee(&self) -> u128 {
        self.txn.get_fee()
    }

    pub fn validator_fee_share(&self) -> u128 {
        self.txn.validator_fee_share()
    }

    pub fn proposer_fee_share(&self) -> u128 {
        self.txn.proposer_fee_share()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Txn {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTxnArgs {
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

impl Default for Txn {
    fn default() -> Self {
        Txn::null_txn()
    }
}

impl Txn {
    pub fn new(args: NewTxnArgs) -> Self {
        let token = args.token.clone().unwrap_or_default();

        let digest_vec = generate_txn_digest_vec(
            args.timestamp,
            args.sender_address.to_string(),
            args.sender_public_key,
            args.receiver_address.to_string(),
            token.clone(),
            args.amount,
            args.nonce,
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

    pub fn null_txn() -> Txn {
        let timestamp = chrono::Utc::now().timestamp();
        let kp = Keypair::random();
        let public_key = kp.miner_kp.1;
        let address = Address::new(public_key);

        let digest_vec = generate_txn_digest_vec(
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

    /// Produces a SHA 256 hash slice of bytes from the transaction
    pub fn id(&self) -> TransactionDigest {
        self.id.clone()
    }

    pub fn build_payload_digest(&self) -> TransactionDigest {
        let digest = generate_txn_digest_vec(
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
    pub fn digest(&self) -> TransactionDigest {
        self.id()
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
        self == &Txn::null_txn()
    }

    pub fn amount(&self) -> TxAmount {
        self.amount
    }

    /// Alias for amount()
    pub fn get_amount(&self) -> TxAmount {
        self.amount()
    }

    pub fn token(&self) -> Token {
        self.token.clone()
    }

    pub fn timestamp(&self) -> TxTimestamp {
        self.timestamp
    }

    pub fn sender_address(&self) -> Address {
        self.sender_address.clone()
    }

    pub fn sender_public_key(&self) -> PublicKey {
        self.sender_public_key
    }

    pub fn receiver_address(&self) -> Address {
        self.receiver_address.clone()
    }

    pub fn signature(&self) -> Signature {
        self.signature
    }

    pub fn nonce(&self) -> TxNonce {
        self.nonce
    }

    pub fn validators(&self) -> HashMap<String, bool> {
        self.validators.clone().unwrap_or_default()
    }

    pub fn generate_txn_digest_vec(&self) -> ByteVec {
        generate_txn_digest_vec(
            self.timestamp(),
            self.sender_address().to_string(),
            self.sender_public_key(),
            self.receiver_address().to_string(),
            self.token(),
            self.amount(),
            self.nonce(),
        )
    }

    pub fn build_payload(&self) -> String {
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

    fn from_byte_slice(data: ByteSlice) -> Self {
        if let Ok(txn) = decode_from_json_byte_slice::<Self>(data) {
            return txn;
        }

        if let Ok(txn) = decode_from_binary_byte_slice::<Self>(data) {
            return txn;
        }

        Txn::null_txn()
    }

    fn from_string(data: &str) -> Txn {
        if let Ok(txn) = Txn::from_str(data) {
            return txn;
        }

        Txn::null_txn()
    }

    pub fn get_fee(&self) -> u128 {
        BASE_FEE
    }

    pub fn validator_fee_share(&self) -> u128 {
        BASE_FEE / 2u128
    }

    pub fn proposer_fee_share(&self) -> u128 {
        BASE_FEE / 2u128
    }

    #[deprecated(note = "will be removed from Txn struct soon")]
    pub fn sign(&mut self, sk: &SecretKey) {
        // TODO: refactor signing out the txn structure definition
        // TODO: return Result<(), SignatureError>
        let message = Message::from_slice(self.build_payload().as_bytes());
        if let Ok(msg) = message {
            let sig = sk.sign_ecdsa(msg);
            self.signature = sig;
        }
    }
}

impl FromStr for Txn {
    type Err = TxnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Txn>(s)
            .map_err(|err| TxnError::InvalidTxn(format!("failed to parse &str into Txn: {err}")))
    }
}

impl fmt::Display for Txn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let txn_ser = serde_json::to_string_pretty(self).unwrap_or_default();

        write!(f, "{}", txn_ser)
    }
}

impl From<String> for Txn {
    fn from(data: String) -> Self {
        Self::from(data.as_str())
    }
}

impl From<Vec<u8>> for Txn {
    fn from(data: Vec<u8>) -> Self {
        Txn::from_byte_slice(&data)
    }
}

impl From<&[u8]> for Txn {
    fn from(data: &[u8]) -> Self {
        Txn::from_byte_slice(data)
    }
}

impl From<&str> for Txn {
    fn from(data: &str) -> Self {
        Self::from_string(data)
    }
}

impl From<Txn> for TransactionDigest {
    fn from(txn: Txn) -> Self {
        txn.id()
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
        self.generate_txn_digest_vec() == other.generate_txn_digest_vec()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTxnArgsError(String);

impl FromStr for NewTxnArgs {
    type Err = ParseTxnArgsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).map_err(|err| ParseTxnArgsError(err.to_string()))
    }
}

pub const TRANSACTION_DIGEST_LENGTH: usize = DIGEST_LENGTH;

#[derive(Debug, Default, Clone, Hash, Deserialize, Serialize, Eq, PartialEq)]
pub struct TransactionDigest {
    inner: PrimitiveDigest,
    digest_string: String,
}

impl TransactionDigest {
    /// Produces a SHA 256 hash string of the transaction
    pub fn digest_string(&self) -> String {
        self.digest_string.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty() && self.digest_string.is_empty()
    }
}

impl Display for TransactionDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.digest_string)
    }
}

impl From<ByteVec> for TransactionDigest {
    fn from(byte_vec: ByteVec) -> Self {
        let digest_string = hex::encode(byte_vec.as_slice());
        let inner = byte_vec.try_into().unwrap_or_default();

        Self {
            inner,
            digest_string,
        }
    }
}

impl<'a> From<ByteSlice<'a>> for TransactionDigest {
    fn from(byte_slice: ByteSlice) -> Self {
        let inner = byte_slice.try_into().unwrap_or_default();

        let digest_string = gen_hex_encoded_string(byte_slice);

        Self {
            inner,
            digest_string,
        }
    }
}

impl FromStr for TransactionDigest {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decoded = hex::decode(s)
            .map_err(|err| Self::Err::Other(format!("failed to decode hex string: {}", err)))?;

        let dec = Self::from(decoded);

        Ok(dec)
    }
}

impl PartialOrd for TransactionDigest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TransactionDigest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.digest_string.cmp(&other.digest_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txn_digest_serde() {
        let txn = Txn::default();

        let txn_digest = txn.id();
        let txn_digest_str = txn_digest.to_string();

        let txn_digest_recovered = txn_digest_str.parse::<TransactionDigest>().unwrap();

        assert_eq!(txn_digest, txn_digest_recovered);
    }
}
