use std::{
    result::Result as StdResult,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use bytebuffer::ByteBuffer;
use left_right::ReadHandle;
use lr_trie::{GetDeserialized, LeftRightTrieError};
use lrdb::Account;
use patriecia::{db::Database, error::TrieError, inner::InnerTrie};
#[allow(deprecated)]
use secp256k1::{
    Signature,
    {Message, PublicKey, Secp256k1},
};
use txn::txn::Txn;

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
    FailedToGetValueForKey(Vec<u8>, TrieError),
    FailedToDeserializeValue(Vec<u8>),
    FailedToSerializeAccount(Account),
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

    /// Verifies Txn signature.
    // TODO, to be moved to a common utility crate
    #[allow(deprecated)]
    pub fn verify_signature(&self, txn: &Txn) -> Result<()> {
        match Signature::from_str(txn.txn_signature.as_str()) {
            Ok(signature) => match PublicKey::from_str(txn.sender_public_key.as_str()) {
                Ok(pk) => {
                    let payload_bytes = txn.txn_payload.as_bytes().to_owned();

                    let mut payload_buffer = ByteBuffer::new();
                    payload_buffer.write_bytes(&payload_bytes);
                    while payload_buffer.len() < 32 {
                        payload_buffer.write_u8(0);
                    }

                    let new_payload = payload_buffer.to_bytes();
                    let payload_hash = blake3::hash(&new_payload);

                    match Message::from_slice(payload_hash.as_bytes()) {
                        Ok(message_hash) => Secp256k1::new()
                            .verify(&message_hash, &signature, &pk)
                            .map_err(|_| TxnValidatorError::TxnSignatureIncorrect),
                        Err(_) => Err(TxnValidatorError::TxnSignatureIncorrect),
                    }
                },
                Err(_) => Err(TxnValidatorError::TxnSignatureIncorrect),
            },
            Err(_) => Err(TxnValidatorError::TxnSignatureIncorrect),
        }
    }

    /// Txn signature validator.
    pub fn validate_signature(&self, txn: &Txn) -> Result<()> {
        if !txn.txn_signature.is_empty() {
            self.verify_signature(txn)
                .map_err(|_| TxnValidatorError::TxnSignatureIncorrect)
        } else {
            Err(TxnValidatorError::TxnSignatureIncorrect)
        }
    }

    /// Txn public key validator
    pub fn validate_public_key(&self, txn: &Txn) -> Result<()> {
        if !txn.sender_public_key.is_empty() {
            match PublicKey::from_str(txn.sender_public_key.as_str()) {
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
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let timestamp = duration.as_nanos();
                if txn.txn_timestamp > 0 && txn.txn_timestamp < timestamp {
                    Ok(())
                } else {
                    Err(TxnValidatorError::TxnTimestampIncorrect)
                }
            },
            Err(_) => Err(TxnValidatorError::TimestampError),
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
                    .checked_sub(txn.txn_amount)
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
