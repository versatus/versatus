use std::{
    collections::{HashMap, HashSet},
    result::Result as StdResult,
    time::{SystemTime, UNIX_EPOCH},
};

use left_right::ReadHandle;
use lr_trie::LeftRightTrieError;
use patriecia::{db::Database, error::TrieError, inner::InnerTrie};
use primitives::types::AccountAddress;
use vrrb_core::{
    account::Account,
    keypair::{KeyPair, MinerPk},
    txn::Txn,
};

pub type Result<T> = StdResult<T, TxnValidatorError>;

pub const ADDRESS_PREFIX: &str = "0x192";

pub enum TxnFees {
    Slow,
    Fast,
    Instant,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Hash)]
pub enum TxnValidatorError {
    #[error("invalid sender")]
    InvalidSender,

    #[error("missing sender address")]
    SenderAddressMissing,

    #[error("invalid sender address")]
    SenderAddressIncorrect,

    #[error("invalid sender public key")]
    SenderPublicKeyIncorrect,

    #[error("missing receiver address")]
    ReceiverAddressMissing,

    #[error("invalid receiver address")]
    ReceiverAddressIncorrect,

    #[error("timestamp {0} is outside of the permitted date range [0, {1}]")]
    OutOfBoundsTimestamp(i64, i64),

    #[error("value {0} is outside of the permitted range [{1}, {2}]")]
    OutOfBounds(String, String, String),

    #[error("invalid amount")]
    TxnAmountIncorrect,

    #[error("invalid signature")]
    TxnSignatureIncorrect,

    #[error("invalid threshold signature")]
    TxnSignatureTresholdIncorrect,

    #[error("value not found")]
    NotFound,

    #[error("account not found within state state_snapshot: {0}")]
    AccountNotFound(String),
}

#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub accounts: HashMap<String, Account>,
}

impl StateSnapshot {
    pub fn new() -> StateSnapshot {
        StateSnapshot {
            accounts: HashMap::new(),
        }
    }

    pub fn get_account(&self, address: &str) -> Result<Account> {
        self.accounts
            .get(address)
            .ok_or(TxnValidatorError::AccountNotFound(address.to_string()))
            .cloned()
    }
}

#[derive(Debug, Clone)]
// TODO: make validator configurable
pub struct TxnValidator {}

impl TxnValidator {
    /// Creates a new Txn validator
    pub fn new() -> TxnValidator {
        TxnValidator {}
    }

    /// An entire Txn validator
    // TODO: include fees and signature threshold.
    pub fn validate(&self, state_snapshot: &StateSnapshot, txn: &Txn) -> Result<()> {
        self.validate_structure(state_snapshot, txn)
    }

    /// An entire Txn structure validator
    pub fn validate_structure(&self, state_snapshot: &StateSnapshot, txn: &Txn) -> Result<()> {
        self.validate_amount(state_snapshot, txn)
            .and_then(|_| self.validate_public_key(txn))
            .and_then(|_| self.validate_sender_address(txn))
            .and_then(|_| self.validate_receiver_address(txn))
            .and_then(|_| self.validate_signature(txn))
            .and_then(|_| self.validate_amount(state_snapshot, txn))
            .and_then(|_| self.validate_timestamp(txn))
    }

    /// Txn signature validator.
    pub fn validate_signature(&self, txn: &Txn) -> Result<()> {
        let txn_signature = txn.signature.clone().unwrap_or_default();
        if !txn_signature.is_empty() {
            KeyPair::verify_ecdsa_sign(
                // TODO: revisit this verification
                format!("{:?}", txn.signature),
                // String::from_slice(&txn.signature),
                // txn.signature.clone(),
                txn.payload().as_bytes(),
                txn.sender_public_key.clone(),
            )
            .map_err(|_| TxnValidatorError::TxnSignatureIncorrect)
        } else {
            Err(TxnValidatorError::TxnSignatureIncorrect)
        }
    }

    /// Txn public key validator
    pub fn validate_public_key(&self, txn: &Txn) -> Result<()> {
        if !txn.sender_public_key.is_empty() {
            match MinerPk::from_slice(&txn.sender_public_key) {
                Ok(_) => Ok(()),
                Err(_) => Err(TxnValidatorError::SenderPublicKeyIncorrect),
            }
        } else {
            Err(TxnValidatorError::SenderPublicKeyIncorrect)
        }
    }

    /// Txn sender validator
    // TODO, to be synchronized with Wallet.
    pub fn validate_sender_address(&self, txn: &Txn) -> Result<()> {
        if !txn.sender_address.is_empty()
            && txn.sender_address.starts_with(ADDRESS_PREFIX)
            && txn.sender_address.len() > 10
        {
            Ok(())
        } else {
            Err(TxnValidatorError::SenderAddressMissing)
        }
    }

    /// Txn receiver validator
    // TODO, to be synchronized with Wallet.
    pub fn validate_receiver_address(&self, txn: &Txn) -> Result<()> {
        if !txn.receiver_address.is_empty()
            && txn.receiver_address.starts_with(ADDRESS_PREFIX)
            && txn.receiver_address.len() > 10
        {
            Ok(())
        } else {
            Err(TxnValidatorError::ReceiverAddressMissing)
        }
    }

    /// Txn timestamp validator
    pub fn validate_timestamp(&self, txn: &Txn) -> Result<()> {
        let timestamp = chrono::offset::Utc::now().timestamp();

        // TODO: revisit seconds vs nanoseconds for timestamp
        // let timestamp = duration.as_nanos();
        if txn.timestamp > 0 && txn.timestamp < timestamp {
            return Ok(());
        } else {
            Err(TxnValidatorError::OutOfBoundsTimestamp(
                txn.timestamp,
                timestamp,
            ))
        }
    }

    /// Txn receiver validator
    // TODO, to be synchronized with transaction fees.
    pub fn validate_amount(&self, state_snapshot: &StateSnapshot, txn: &Txn) -> Result<()> {
        let address = txn.sender_address.clone();

        let account = state_snapshot.get_account(&address)?;

        if (account.credits - account.debits)
            .checked_sub(txn.amount())
            .is_none()
        {
            return Err(TxnValidatorError::TxnAmountIncorrect);
        };

        Ok(())
    }
}
