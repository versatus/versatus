use std::{collections::HashMap, result::Result as StdResult, str::FromStr};

use mempool::MempoolReadHandleFactory;
use primitives::Address;
use storage::vrrbdb::{StateStoreReadHandle, StateStoreReadHandleFactory};
use vrrb_core::transactions::{Transaction, TransactionKind};
use vrrb_core::{account::Account, keypair::KeyPair};
use sha2::{Sha256, Digest};

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
    TxnSignatureIncorrect(String),

    #[error("invalid threshold signature")]
    TxnSignatureTresholdIncorrect,

    #[error("value not found")]
    NotFound,

    #[error("account not found within state state_snapshot: {0}")]
    AccountNotFound(String),
    #[error("transaction payload not valid")]
    PayloadInvalid(String),
    #[error("other")]
    Other(String)
}

#[derive(Debug, Clone, Default)]
// TODO: make validator configurable
pub struct TxnValidator;

impl TxnValidator {
    /// Creates a new Txn validator
    pub fn new() -> TxnValidator {
        TxnValidator
    }

    /// An entire Txn validator
    // TODO: include fees and signature threshold.
    pub fn validate(
        &self,
        state_reader: StateStoreReadHandleFactory,
        txn: &TransactionKind,
    ) -> Result<()> {
        self.validate_structure(state_reader, txn)
    }

    /// An entire Txn structure validator
    pub fn validate_structure(
        &self,
        state_reader: StateStoreReadHandleFactory,
        txn: &TransactionKind,
    ) -> Result<()> {
        self.validate_amount(state_reader, txn)
            .and_then(|_| self.validate_public_key(txn))
 //           .and_then(|_| self.validate_sender_address(txn))
 //           .and_then(|_| self.validate_receiver_address(txn))
            .and_then(|_| self.validate_signature(txn))
            .and_then(|_| self.validate_timestamp(txn))
    }

    /// Txn signature validator.
    pub fn validate_signature(&self, txn: &TransactionKind) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(txn.build_payload().as_bytes());
        let result = hasher.finalize().to_vec();
        let message = secp256k1::Message::from_slice(&result).map_err(|err| {
            TxnValidatorError::PayloadInvalid(err.to_string())
        })?;
        txn.signature().verify(&message, &txn.sender_public_key()).map_err(|err| {
            TxnValidatorError::TxnSignatureIncorrect(err.to_string())
        })
    }

    /// Txn public key validator
    pub fn validate_public_key(&self, txn: &TransactionKind) -> Result<()> {
        if !txn.sender_public_key().to_string().is_empty() {
            Ok(())
        } else {
            Err(TxnValidatorError::SenderPublicKeyIncorrect)
        }
    }

    /// Txn sender validator
    // TODO, to be synchronized with Wallet.
    // pub fn validate_sender_address(&self, txn: &TransactionKind) -> Result<()> {
    //    if !txn.sender_address().to_string().is_empty()
    //        && txn.sender_address().to_string().starts_with(ADDRESS_PREFIX)
    //        && txn.sender_address().to_string().len() > 10
    //    {
    //        Ok(())
    //    } else {
    //        Err(TxnValidatorError::SenderAddressMissing)
    //    }
    // }

    /// Txn receiver validator
    // TODO, to be synchronized with Wallet.
//    pub fn validate_receiver_address(&self, txn: &TransactionKind) -> Result<()> {
//        if !txn.receiver_address().to_string().is_empty()
//            && txn
//                .receiver_address()
//                .to_string()
//                .starts_with(ADDRESS_PREFIX)
//            && txn.receiver_address().to_string().len() > 10
//        {
//            Ok(())
//        } else {
//            Err(TxnValidatorError::ReceiverAddressMissing)
//        }
//    }

    /// Txn timestamp validator
    pub fn validate_timestamp(&self, txn: &TransactionKind) -> Result<()> {
        let timestamp = chrono::offset::Utc::now().timestamp();

        // TODO: revisit seconds vs nanoseconds for timestamp
        // let timestamp = duration.as_nanos();
        if txn.timestamp() > 0 && txn.timestamp() <= timestamp {
            Ok(())
        } else {
            Err(TxnValidatorError::OutOfBoundsTimestamp(
                txn.timestamp(),
                timestamp,
            ))
        }
    }

    /// Txn receiver validator
    // TODO, to be synchronized with transaction fees.
    pub fn validate_amount(
        &self,
        state_reader: StateStoreReadHandleFactory,
        txn: &TransactionKind,
    ) -> Result<()> {
        let address = txn.sender_address();
        let account = state_reader.handle().get(&address)
            .map_err(|_| TxnValidatorError::SenderAddressIncorrect)?;
        if (account.credits() - account.debits())
            .checked_sub(txn.amount())
            .is_none()
        {
            return Err(TxnValidatorError::TxnAmountIncorrect);
        };

        Ok(())
    }
}
