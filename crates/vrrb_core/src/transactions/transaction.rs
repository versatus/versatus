use std::collections::HashMap;
use std::hash::Hash;
use serde::{Deserialize, Serialize};
use primitives::{Address, PublicKey, Signature};
use crate::transactions::{Token, TransactionDigest, TransactionKind, TxAmount, TxNonce, TxTimestamp};


pub trait Transaction<'a>: Clone + Sized + Serialize + Default + Deserialize<'a> {
    fn kind(&self) -> TransactionKind;
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

    #[deprecated]
    fn digest(&self) -> TransactionDigest;
}