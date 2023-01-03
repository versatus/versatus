use std::{
    result::Result as StdResult,
    time::{SystemTime, UNIX_EPOCH},
};

use left_right::ReadHandle;
use lr_trie::{GetDeserialized, LeftRightTrieError};
use lrdb::Account;
use patriecia::{db::Database, error::TrieError, inner::InnerTrie};
use vrrb_core::{
    keypair::{KeyPair, MinerPk},
    txn::Txn,
};

type Result<T> = StdResult<T, TxnValidatorError>;

pub const ADDRESS_PREFIX: &str = "0x192";
pub enum TxnFees {
    Slow,
    Fast,
    Instant,
}

#[derive(PartialEq, Eq, Debug)]
pub enum TxnValidatorError {
    InvalidSender,
    SenderAddressMissing,
    SenderAddressIncorrect,
    SenderPublicKeyIncorrect,
    ReceiverAddressMissing,
    ReceiverAddressIncorrect,
    TxnIdIncorrect,
    TxnTimestampIncorrect,
    TxnAmountIncorrect,
    TxnSignatureIncorrect,
    TxnSignatureTresholdIncorrect,
    TimestampError,
    FailedToGetValueForKey(TrieError),
    FailedToDeserializeValue,
    FailedToSerializeAccount,
    NoValueForKey,
}

#[derive(Debug, Clone)]
pub struct TxnValidator<D: Database> {
    pub state: ReadHandle<InnerTrie<D>>,
}
impl<D: Database> TxnValidator<D> {
    /// Creates a new Txn validator
    pub fn new(network_state: ReadHandle<InnerTrie<D>>) -> TxnValidator<D> {
        TxnValidator {
            state: network_state,
        }
    }

    /// Txn signature validator.
    pub fn validate_signature(&self, txn: &Txn) -> Result<()> {
        if !txn.signature.is_empty() {
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

        // let timestamp = duration.as_nanos();
        if txn.timestamp > 0 && txn.timestamp < timestamp {
            return Ok(());
        } else {
            Err(TxnValidatorError::TxnTimestampIncorrect)
        }
    }

    /// Txn receiver validator
    // TODO, to be synchronized with transaction fees.
    pub fn validate_amount(&self, txn: &Txn) -> Result<()> {
        let data: StdResult<Account, LeftRightTrieError> = self
            .state
            .get_deserialized_data(txn.sender_address.clone().into_bytes());
        match data {
            Ok(account) => {
                if (account.credits - account.debits)
                    .checked_sub(txn.amount())
                    .is_none()
                {
                    return Err(TxnValidatorError::TxnAmountIncorrect);
                };
                Ok(())
            },
            Err(_) => Err(TxnValidatorError::InvalidSender),
        }
    }

    /// An entire Txn structure validator
    pub fn validate_structure(&self, txn: &Txn) -> Result<()> {
        self.validate_amount(txn)
            .and_then(|_| self.validate_public_key(txn))
            .and_then(|_| self.validate_sender_address(txn))
            .and_then(|_| self.validate_receiver_address(txn))
            .and_then(|_| self.validate_signature(txn))
            .and_then(|_| self.validate_amount(txn))
            .and_then(|_| self.validate_timestamp(txn))
    }

    /// An entire Txn validator
    // TODO: include fees and signature threshold.
    pub fn validate(&self, txn: &Txn) -> Result<()> {
        self.validate_structure(txn)
    }
}
