use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    str::FromStr,
};

use primitives::{
    ByteSlice,
    ByteVec,
    Digest as PrimitiveDigest,
    PublicKey,
    SecretKey,
    DIGEST_LENGTH,
};
use secp256k1::{ecdsa::Signature, Message, Secp256k1};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sha256::digest;
use utils::hash_data;

use crate::{
    helpers::gen_hex_encoded_string,
    serde_helpers::{
        decode_from_binary_byte_slice,
        decode_from_json_byte_slice,
        encode_to_binary,
        encode_to_json,
    },
};
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Txn {
    pub id: TransactionDigest,
    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: PublicKey,
    pub receiver_address: String,
    pub token: Token,
    pub amount: TxAmount,
    pub signature: Signature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTxnArgs {
    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: PublicKey,
    pub receiver_address: String,
    pub token: Option<Token>,
    pub amount: TxAmount,
    pub signature: Signature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
}

impl Default for Txn {
    fn default() -> Self {
        null_txn()
    }
}

impl Txn {
    pub fn new(args: NewTxnArgs) -> Self {
        let token = args.token.clone().unwrap_or_default();

        let digest_vec = generate_txn_digest_vec(
            args.timestamp,
            args.sender_address.clone(),
            args.sender_public_key,
            args.receiver_address.clone(),
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

    /// Produces a SHA 256 hash slice of bytes from the transaction
    pub fn id(&self) -> TransactionDigest {
        self.id.clone()
    }

    pub fn build_payload_digest(&self) -> TransactionDigest {
        let digest = generate_txn_digest_vec(
            self.timestamp(),
            self.sender_address(),
            self.sender_public_key(),
            self.receiver_address(),
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
        self == &null_txn()
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
        self.timestamp.clone()
    }

    pub fn sender_address(&self) -> String {
        self.sender_address.clone()
    }

    pub fn sender_public_key(&self) -> PublicKey {
        self.sender_public_key
    }

    pub fn receiver_address(&self) -> String {
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
            self.sender_address(),
            self.sender_public_key(),
            self.receiver_address(),
            self.token(),
            self.amount(),
            self.nonce(),
        )
    }

    pub fn build_payload(&self) -> String {
        hash_data!(
            self.sender_address.clone(),
            self.sender_public_key.clone(),
            self.receiver_address.clone(),
            self.token.clone(),
            self.amount.clone(),
            self.nonce.clone()
        )
    }

    fn from_byte_slice(data: ByteSlice) -> Self {
        if let Ok(result) = decode_from_json_byte_slice::<Self>(data) {
            return result;
        }

        if let Ok(result) = decode_from_binary_byte_slice::<Self>(data) {
            return result;
        }

        null_txn()
    }

    fn from_string(data: &str) -> Txn {
        Txn::from_str(data).unwrap_or(null_txn())
    }

    #[deprecated(note = "will be removed from Txn struct soon")]
    pub fn sign(&mut self, sk: &SecretKey) {
        // TODO: refactor signing out the txn structure definition
        if let payload = self.build_payload() {
            let message = Message::from_slice(payload.as_bytes());
            match message {
                Ok(msg) => {
                    let sig = sk.sign_ecdsa(msg);
                    self.signature = sig.into();
                },
                _ => { /*TODO return Result<(), SignatureError>*/ },
            }
        } else {
            self.build_payload();
            self.sign(sk);
        }
    }
}

/// Returns a null transaction
pub fn null_txn() -> Txn {
    type H = secp256k1::hashes::sha256::Hash;

    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_hashed_data::<H>(b"vrrb");
    let sender_public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let message = Message::from_hashed_data::<H>(b"vrrb");
    let signature = secp.sign_ecdsa(&message, &secret_key);

    let txn_digest_vec = generate_txn_digest_vec(
        0,
        String::new(),
        sender_public_key,
        String::new(),
        Token::default(),
        0,
        0,
    );

    let digest = TransactionDigest::from(txn_digest_vec);

    Txn {
        id: digest,
        timestamp: 0,
        sender_address: String::new(),
        sender_public_key,
        receiver_address: String::new(),
        token: Token::default(),
        amount: 0,
        signature,
        validators: None,
        nonce: 0,
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

impl FromStr for Txn {
    type Err = TxnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Txn>(s)
            .map_err(|err| TxnError::InvalidTxn(format!("failed to parse &str into Txn: {err}")))
    }
}

impl From<&str> for Txn {
    fn from(data: &str) -> Self {
        Self::from_string(data)
    }
}

impl From<Txn> for TransactionDigest {
    fn from(txn: Txn) -> Self {
        txn.digest()
    }
}

impl fmt::Display for Txn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let txn_ser = serde_json::to_string_pretty(self).unwrap_or_default();

        write!(f, "{}", txn_ser)
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
        self.digest() == other.digest()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txn_digest_serde() {
        let txn = Txn::default();

        let txn_digest = txn.digest();
        let txn_digest_str = txn_digest.to_string();

        let txn_digest_recovered = txn_digest_str.parse::<TransactionDigest>().unwrap();

        assert_eq!(txn_digest, txn_digest_recovered);
    }
}
