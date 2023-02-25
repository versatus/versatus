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
    SerializedPublicKey,
    SerializedPublicKeyString,
    DIGEST_LENGTH,
};
use secp256k1::{ecdsa::Signature, Message, Secp256k1};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sha256::digest;
use utils::hash_data;

/// This module contains the basic structure of simple transaction
use crate::{
    accountable::Accountable,
    helpers::gen_sha256_digest_string,
    result,
    serde_helpers::{decode_from_binary_byte_slice, decode_from_json_byte_slice, encode_to_binary},
};
use crate::{serde_helpers::encode_to_json, verifiable::Verifiable};

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

/// The basic transation structure.
//TODO: Discuss the pieces of the Transaction structure that should stay and go
//TODO: Discuss how to best package this to minimize the size of it/compress it
//TODO: Change `validators` filed to `receipt` or `certificate` to put threshold
//signature of validators in.
#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub struct Txn {
    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: PublicKey,
    pub receiver_address: String,
    token: Token,
    amount: TxAmount,
    pub payload: Option<TxPayload>,
    pub signature: Signature,
    pub validators: Option<HashMap<String, bool>>,
    pub nonce: TxNonce,
    pub receiver_farmer_id: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTxnArgs {
    pub timestamp: TxTimestamp,
    pub sender_address: String,
    pub sender_public_key: PublicKey,
    pub receiver_address: String,
    pub token: Option<Token>,
    pub amount: TxAmount,
    pub payload: Option<TxPayload>,
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

        Self {
            // TODO: change time unit from seconds to millis
            timestamp: args.timestamp,
            sender_address: args.sender_address,
            sender_public_key: args.sender_public_key,
            receiver_address: args.receiver_address,
            token,
            amount: args.amount,
            payload: args.payload,
            signature: args.signature,
            validators: args.validators,
            nonce: args.nonce,
            receiver_farmer_id: None,
        }
    }

    /// Produces a SHA 256 hash string of the transaction
    pub fn digest_string(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.encode_to_string());
        let hash = hasher.finalize();

        // NOTE: it's the same as hashing and then calling hex::encode
        gen_sha256_digest_string(&hash[..])
    }

    pub fn digest_vec(&self) -> ByteVec {
        let mut hasher = Sha256::new();
        hasher.update(self.encode_to_string());
        let hash = hasher.finalize();

        hash.to_vec()
    }

    pub fn encode_to_string(&self) -> String {
        format!(
            "{},{},{},{},{},{:?},{}",
            &self.timestamp,
            &self.sender_address,
            &self.sender_public_key,
            &self.receiver_address,
            &self.amount,
            &self.token,
            &self.nonce.clone()
        )
    }

    /// Produces a SHA 256 hash slice of bytes from the transaction
    pub fn digest(&self) -> TransactionDigest {
        TransactionDigest::from(self.digest_vec())
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

    pub fn set_token(&mut self, token: Token) {
        self.token = token;
    }

    pub fn set_amount(&mut self, amount: u128) {
        self.amount = amount;
    }

    pub fn validators(&self) -> HashMap<String, bool> {
        self.validators.clone().unwrap_or_default()
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

    fn from_byte_slice(data: ByteSlice) -> Self {
        if let Ok(result) = decode_from_json_byte_slice::<Self>(data) {
            return result;
        }

        if let Ok(result) = decode_from_binary_byte_slice::<Self>(data) {
            return result;
        }

        null_txn()
    }

    #[deprecated(note = "use digest instead")]
    pub fn txn_id(&self) -> String {
        self.digest_string()
    }

    fn from_string(data: &str) -> Txn {
        Txn::from_str(data).unwrap_or(null_txn())
    }

    pub fn sign(&mut self, sk: &SecretKey) {
        if let Some(payload) = self.payload.clone() {
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

    Txn {
        timestamp: 0,
        sender_address: String::new(),
        // sender_public_key: PublicKey::from_slice(NULL_SENDER_PUBLIC_KEY_SLICE).unwrap(),
        sender_public_key,
        receiver_address: String::new(),
        token: Token::default(),
        amount: 0,
        payload: None,
        signature,
        validators: None,
        nonce: 0,
        receiver_farmer_id: None,
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
        _item: &Self::Item,
        _debendencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error> {
        Ok(true)
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
    pub fn new(txn: &Txn) -> Self {
        Self {
            inner: PrimitiveDigest::from(txn.digest_vec()),
            digest_string: txn.digest_string(),
        }
    }

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
        let digest_string = gen_sha256_digest_string(byte_vec.as_slice());
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

        let digest_string = gen_sha256_digest_string(byte_slice);

        Self {
            inner,
            digest_string,
        }
    }
}
