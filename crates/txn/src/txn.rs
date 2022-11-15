#![allow(unused_imports, dead_code, deprecated)]
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fmt::{self, Display},
    hash::{Hash, Hasher},
    ops::{Add, AddAssign, Sub},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// This module contains the basic structure of simple transaction
use accountable::accountable::Accountable;
use bytebuffer::ByteBuffer;
use pool::pool::Pool;
use primitives::types::{Message, PublicKey, RawSignature, Secp256k1, SecretKey, Signature};
use secp256k1::All;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use state::state::NetworkState;
use uuid::Uuid;
use verifiable::verifiable::Verifiable;

/// A simple custom error type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvalidTxnError {
    pub details: String,
}

/// Amount of OBOL in one VRRB token
pub const OBOLS_IN_VRRB: u128 = 1_000_000_000;

/// TxnPriority decides how priorities given txn will be.
/// The associated fee will be added to txn overall cost for each txn
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum TxnPriority {
    Slow,
    Fast,
    Instant,
    Contract,
}

impl Default for TxnPriority {
    fn default() -> Self {
        TxnPriority::Slow
    }
}


///
impl From<TxnPriority> for Obol {
    fn from(priority: TxnPriority) -> Self {
        match priority {
            TxnPriority::Slow => Obol(OBOLS_IN_VRRB / 100),
            TxnPriority::Fast => Obol(OBOLS_IN_VRRB / 20),
            TxnPriority::Instant | TxnPriority::Contract => Obol(OBOLS_IN_VRRB / 10),
        }
    }
}


#[derive(Debug, Eq, PartialEq)]
pub enum SystemTokenError {
    ConversionError,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Vrrb(pub u128);
impl TryFrom<Obol> for Vrrb {
    type Error = SystemTokenError;

    fn try_from(value: Obol) -> Result<Self, Self::Error> {
        if value.0 % OBOLS_IN_VRRB != 0 {
            return Err(SystemTokenError::ConversionError);
        }
        match value.0.checked_div(OBOLS_IN_VRRB) {
            Some(res) => Ok(Self(res)),
            None => Err(SystemTokenError::ConversionError),
        }
    }
}

impl TryFrom<Vrrb> for Obol {
    type Error = SystemTokenError;

    fn try_from(value: Vrrb) -> Result<Self, Self::Error> {
        match value.0.checked_mul(OBOLS_IN_VRRB) {
            Some(res) => Ok(Self(res)),
            None => Err(SystemTokenError::ConversionError),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Obol(pub u128);


impl Add for Obol {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add for Vrrb {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Vrrb {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Sub for Obol {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign for Obol {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0) + rhs;
    }
}

impl AddAssign for Vrrb {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0) + rhs;
    }
}


/// The basic transation structure.
//TODO: Discuss the pieces of the Transaction structure that should stay and go
//TODO: Discuss how to best package this to minimize the size of it/compress it
//TODO: Change `validators` filed to `receipt` or `certificate` to put
// threshold signature of validators in.

// Instruction: Transfer
// Instruction: Deploy

/// SystemInstruction enum represents all possible system instructions
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemInstruction {
    Transfer(TransferData),
    ContractDeploy(Code),
    ContractUpgrade(Code),
    ContractCall(CallData),
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct NativeToken(pub u128);
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Code(pub Vec<u8>);
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct CallData(pub Vec<u8>);

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct TransferData {
    pub amount: NativeToken,
    pub from: PublicKey,
    pub to: PublicKey,
}


/// Transaction struct represensing the transactions to be sent over the network
///
/// `id` - id
///
/// `sender` - public key of the sender of the message.
///  Should match with the first signature of the transaction
///
/// `signature` - contains signature for the message, signing the `instruction`
/// and `sender` fields.
// TODO: Validating vector of instructions means, the validator would process
// all of them, as validating n+1 th instruction require state changes of 0..n
// txns to be applied. To avoid situation where validators are DOSed with big
// amount of following malicious txns - n valid instructions and n+1th
// instruction malicious - resulting in validator computing n instructions
// without receiving the fee (as txn is deemed  invalid) Prevention should be
// discussed. RPC prevalidating the txn could help with the problem, but wouldnt
// solve 100% of it
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Transaction {
    // id: String,
    pub instructions: Vec<SystemInstruction>,
    pub sender: PublicKey,
    pub signature: Option<Signature>,
    pub receipt: Vec<(RawSignature, bool)>,
    pub priority: TxnPriority,
}

impl Transaction {
    pub fn sign(&mut self, secret: &SecretKey) -> Result<(), InvalidTxnError> {
        let secp = Secp256k1::new();
        self.signature = Some(secp.sign_ecdsa(&self.get_message()?, secret));
        Ok(())
    }

    pub fn get_id(&self) -> String {
        let mut hash = DefaultHasher::new();
        self.hash(&mut hash);
        format!("{:x}", hash.finish())
    }

    fn get_message(&self) -> Result<Message, InvalidTxnError> {
        let mut msg_bytes = vec![];

        let bytes = serde_json::to_vec(&self.sender).map_err(|e| InvalidTxnError {
            details: e.to_string(),
        })?;

        msg_bytes.extend(bytes);

        let bytes = serde_json::to_vec(&self.priority).map_err(|e| InvalidTxnError {
            details: e.to_string(),
        })?;

        msg_bytes.extend(bytes);

        let bytes = serde_json::to_vec(&self.instructions).map_err(|e| InvalidTxnError {
            details: e.to_string(),
        })?;

        msg_bytes.extend(bytes);

        let mut hasher = Sha256::new();
        hasher.update(msg_bytes);
        let hash_bytes = hasher.finalize().to_vec();

        Message::from_slice(&hash_bytes).map_err(|e| InvalidTxnError {
            details: e.to_string(),
        })
    }

    pub fn verify_signature(&self) -> Result<(), InvalidTxnError> {
        let msg = self.get_message()?;
        let secp = Secp256k1::new();
        if let Some(sig) = self.signature {
            secp.verify_ecdsa(&msg, &sig, &self.sender)
                .map_err(|e| InvalidTxnError {
                    details: e.to_string(),
                })
        } else {
            Err(InvalidTxnError {
                details: "No signature provided".to_string(),
            })
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_string(self).map(|string| string.as_bytes().to_vec())
    }

    pub fn from_string(string: &str) -> Self {
        serde_json::from_str(string).unwrap()
    }

    /// Get amount of VRRB used in the transaction,
    pub fn get_amount(&self) -> u128 {
        let mut amount = 0;

        for ix in &self.instructions {
            amount += match ix {
                SystemInstruction::Transfer(TransferData {
                    amount: NativeToken(amount),
                    ..
                }) => *amount,
                _ => 0,
            };
        }

        amount
    }

    pub fn get_fees(&self) -> Obol {
        let mut fees = Obol(0);


        for ix in &self.instructions {
            fees += match ix {
                SystemInstruction::Transfer(_) => self.priority.clone().into(),
                _ => TxnPriority::Contract.into(),
            }
        }

        fees
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl Default for Transaction {
    fn default() -> Self {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let secret = SecretKey::new(&mut rng);
        Self {
            instructions: Default::default(),
            sender: PublicKey::from_secret_key(&secp, &secret),
            signature: Default::default(),
            receipt: Default::default(),
            priority: Default::default(),
        }
    }
}


#[deprecated = "Replaced with `Transaction` struct"]
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

#[allow(deprecated)]
impl Txn {
    // TODO: convert to_message into a function of the verifiable trait,
    // all verifiable objects need to be able to be converted to a message.

    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let as_string = serde_json::to_string(self).unwrap();
        as_string.as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Txn {
        serde_json::from_slice::<Txn>(data).unwrap()
    }

    pub fn from_string(string: &str) -> Txn {
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

#[allow(deprecated)]
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

#[allow(deprecated)]
impl Verifiable for Txn {
    type Dependencies = (NetworkState, Pool<String, Txn>);
    type Error = InvalidTxnError;
    type Item = Option<String>;

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

#[allow(deprecated)]
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
            self.txn_timestamp,
            self.sender_address,
            self.sender_public_key,
            self.receiver_address,
            self.txn_token,
            self.txn_amount,
            self.txn_signature,
        )
    }
}

#[allow(deprecated)]
impl Hash for Txn {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.txn_id.hash(state);
        self.txn_timestamp.hash(state);
        self.sender_address.hash(state);
        self.sender_public_key.hash(state);
        self.receiver_address.hash(state);
        self.txn_token.hash(state);
        self.txn_amount.hash(state);
        self.txn_payload.hash(state);
        self.txn_signature.hash(state);
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

#[allow(deprecated)]
impl PartialEq for Txn {
    fn eq(&self, other: &Self) -> bool {
        self.txn_id == other.txn_id
            && self.txn_timestamp == other.txn_timestamp
            && self.sender_address == other.sender_address
            && self.sender_public_key == other.sender_public_key
            && self.receiver_address == other.receiver_address
            && self.txn_token == other.txn_token
            && self.txn_amount == other.txn_amount
            && self.txn_signature == other.txn_signature
            && self.nonce == other.nonce
    }
}

#[allow(deprecated)]
impl Eq for Txn {}
