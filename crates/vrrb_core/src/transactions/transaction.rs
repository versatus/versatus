use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use primitives::{Address, ByteSlice, ByteVec, Digest as PrimitiveDigest, DIGEST_LENGTH, NodeIdx, PublicKey, RawSignature, SecretKey, Signature};
use crate::helpers::gen_hex_encoded_string;
use crate::transactions::{TransactionKind, TxAmount, TxNonce, TxTimestamp};

pub const BASE_FEE: u128 = 0x2D79883D2000;

pub trait Transaction {
    fn id(&self) -> TransactionDigest;
    fn timestamp(&self) -> TxTimestamp;
    fn sender_address(&self) -> Address;
    fn sender_public_key(&self) -> PublicKey;
    fn receiver_address(&self) -> Address;
    fn token(&self) -> Token;
    fn amount(&self) -> TxAmount;
    fn signature(&self) -> Signature;
    fn validators(&self) -> Option<HashMap<String, bool>>;
    fn nonce(&self) -> TxNonce;
    fn fee(&self) -> u128;
    fn validator_fee_share(&self) -> u128;
    fn proposer_fee_share(&self) -> u128;

    fn build_payload(&self) -> String;

    #[deprecated]
    fn digest(&self) -> TransactionDigest;

    fn sign(&mut self, sk: &SecretKey);
}

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
pub struct QuorumCertifiedTxn
{
    sender_farmer_id: Vec<u8>,
    /// All valid vote receipts
    votes: Vec<VoteReceipt>,
    txn: TransactionKind,
    /// Threshold Signature
    signature: RawSignature,
    pub is_txn_valid: bool,
}

impl QuorumCertifiedTxn
{
    pub fn new(
        sender_farmer_id: Vec<u8>,
        votes: Vec<VoteReceipt>,
        txn: TransactionKind,
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

    pub fn txn(&self) -> TransactionKind {
        self.txn.clone()
    }

    pub fn fee(&self) -> u128 {
        self.txn.fee()
    }

    pub fn validator_fee_share(&self) -> u128 {
        self.txn.validator_fee_share()
    }

    pub fn proposer_fee_share(&self) -> u128 {
        self.txn.proposer_fee_share()
    }
}

pub type RpcTransactionDigest = String;

#[derive(Debug, Default, Clone, Hash, Deserialize, Serialize, Eq, PartialEq)]
pub struct TransactionDigest {
    inner: PrimitiveDigest,
    digest_string: RpcTransactionDigest,
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

pub const TRANSACTION_DIGEST_LENGTH: usize = DIGEST_LENGTH;

#[cfg(test)]
mod tests {
    use crate::transactions::{TransactionDigest, Transfer};
    use super::*;

    #[test]
    fn test_txn_digest_serde() {
        let txn = Transfer::default();

        let txn_digest = txn.id();
        let txn_digest_str = txn_digest.to_string();

        let txn_digest_recovered = txn_digest_str.parse::<TransactionDigest>().unwrap();

        assert_eq!(txn_digest, txn_digest_recovered);
    }
}
