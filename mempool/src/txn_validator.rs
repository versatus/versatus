use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

// TODO: replace the deprecated secp256k1::Signature
// use ecdsa::Signature;
use bytebuffer::ByteBuffer;
#[allow(deprecated)]
use secp256k1::{
    Signature,
    {Message, PublicKey, Secp256k1},
};
use state::state::NetworkState;
use txn::txn::Txn;

pub const ADDRESS_PREFIX: &str = "0x192";

// TODO: a temporary implementation, to be refactored.

pub enum TxnFees {
    Slow,
    Fast,
    Instant,
}

#[derive(PartialEq, Eq, Debug)]
pub enum TxnValidatorError {
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
}

#[derive(Debug, Clone)]
pub struct TxnValidator<'m> {
    pub txn: Txn,
    pub state: &'m NetworkState,
}

impl<'m> TxnValidator<'_> {
    /// Creates a new Txn validator
    pub fn new(txn: &Txn, network_state: &'m NetworkState) -> TxnValidator<'m> {
        TxnValidator {
            txn: txn.clone(),
            state: network_state,
        }
    }

    /// Verifies Txn signature.
    // TODO, to be moved to a common utility crate
    #[allow(deprecated)]
    pub fn verify_signature(&mut self) -> Result<(), TxnValidatorError> {
        match Signature::from_str(self.txn.txn_signature.as_str()) {
            Ok(signature) => match PublicKey::from_str(self.txn.sender_public_key.as_str()) {
                Ok(pk) => {
                    let payload_bytes = self.txn.txn_payload.as_bytes().to_owned();

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
    pub fn validate_signature(&mut self) -> Result<(), TxnValidatorError> {
        if !self.txn.txn_signature.is_empty() {
            self.verify_signature()
                .map_err(|_| TxnValidatorError::TxnSignatureIncorrect)
        } else {
            Err(TxnValidatorError::TxnSignatureIncorrect)
        }
    }

    /// Txn public key validator
    pub fn validate_public_key(&mut self) -> Result<(), TxnValidatorError> {
        if !self.txn.sender_public_key.is_empty() {
            match PublicKey::from_str(self.txn.sender_public_key.as_str()) {
                Ok(_) => Ok(()),
                Err(_) => Err(TxnValidatorError::SenderPublicKeyIncorrect),
            }
        } else {
            Err(TxnValidatorError::SenderPublicKeyIncorrect)
        }
    }

    /// Txn sender validator
    // TODO, to be synchronized with Wallet.
    pub fn validate_sender_address(&mut self) -> Result<(), TxnValidatorError> {
        if !self.txn.sender_address.is_empty()
            && self.txn.sender_address.starts_with(ADDRESS_PREFIX)
            && self.txn.sender_address.len() > 10
        {
            Ok(())
        } else {
            Err(TxnValidatorError::SenderAddressMissing)
        }
    }

    /// Txn receiver validator
    // TODO, to be synchronized with Wallet.
    pub fn validate_receiver_address(&mut self) -> Result<(), TxnValidatorError> {
        if !self.txn.receiver_address.is_empty()
            && self.txn.receiver_address.starts_with(ADDRESS_PREFIX)
            && self.txn.receiver_address.len() > 10
        {
            Ok(())
        } else {
            Err(TxnValidatorError::ReceiverAddressMissing)
        }
    }

    /// Txn timestamp validator
    pub fn validate_timestamp(&mut self) -> Result<(), TxnValidatorError> {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let timestamp = duration.as_nanos();
                if self.txn.txn_timestamp > 0 && self.txn.txn_timestamp < timestamp {
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
    pub fn validate_amount(&mut self) -> Result<(), TxnValidatorError> {
        if (self.state.get_balance(self.txn.sender_address.as_str()) - self.txn.txn_amount) > 0 {
            Ok(())
        } else {
            Err(TxnValidatorError::TxnAmountIncorrect)
        }
    }

    /// An entire Txn structure validator
    pub fn validate_structure(&mut self) -> Result<(), TxnValidatorError> {
        self.validate_amount()
            .and_then(|_| self.validate_public_key())
            .and_then(|_| self.validate_sender_address())
            .and_then(|_| self.validate_receiver_address())
            .and_then(|_| self.validate_signature())
            .and_then(|_| self.validate_amount())
            .and_then(|_| self.validate_timestamp())
    }

    /// An entire Txn validator
    // TODO: include fees and signature threshold.
    pub fn validate(&mut self) -> Result<(), TxnValidatorError> {
        self.validate_structure()
    }
}
