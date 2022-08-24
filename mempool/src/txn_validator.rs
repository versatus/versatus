use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use txn::txn::Txn;

// TODO: a temporary implementation, to be refactored.

pub enum TxnFees {
    Slow,
    Fast,
    Instant
}

pub type TxnBoxed = Box<Txn>;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnValidator {
    pub txn: Txn
}

impl TxnValidator {

    pub fn new(txn: &Txn) -> TxnValidator {

        TxnValidator {
            txn: txn.clone()
        }
    }

    pub fn validate_fees(&mut self, fees: &TxnFees) -> Result<(), TxnValidatorError> {

        match fees {
            TxnFees::Slow => {
                Ok(())
            },
            TxnFees::Fast => {
                Ok(())
            },
            TxnFees::Instant => {
                Ok(())
            },
        }
    }

    pub fn validate_signature(&mut self) -> Result<(), TxnValidatorError> {

        if ! self.txn.txn_signature.is_empty() {
            Ok(())
        } else {
            Err(TxnValidatorError::TxnSignatureIncorrect)
        }
    }

    pub fn validate_public_key(&mut self) -> Result<(), TxnValidatorError> {

        if ! self.txn.sender_public_key.is_empty() {
            Ok(())
        } else {
            Err(TxnValidatorError::SenderPublicKeyIncorrect)
        }
    }

    pub fn validate_sender_address(&mut self) -> Result<(), TxnValidatorError> {

        if ! self.txn.sender_address.is_empty() {
            Ok(())
        } else {
            Err(TxnValidatorError::SenderAddressMissing)
        }
    }

    pub fn validate_receiver_address(&mut self) -> Result<(), TxnValidatorError> {

        if ! self.txn.receiver_address.is_empty() {
            Ok(())
        } else {
            Err(TxnValidatorError::ReceiverAddressMissing)
        }
    }

    pub fn validate_timestamp(&mut self) -> Result<(), TxnValidatorError> {

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        if  self.txn.txn_timestamp > 0 &&
            self.txn.txn_timestamp < timestamp {
            Ok(())
        } else {
            Err(TxnValidatorError::TxnTimestampIncorrect)
        }
    }

    pub fn validate_amount(&mut self) -> Result<(), TxnValidatorError> {

        if self.txn.txn_amount > 0 {
            Ok(())
        } else {
            Err(TxnValidatorError::TxnAmountIncorrect)
        }
    }

    pub fn validate_structure(&mut self) -> Result<(), TxnValidatorError> {

        self.validate_amount()
            .and_then(|_| self.validate_public_key())
            .and_then(|_| self.validate_sender_address())
            .and_then(|_| self.validate_receiver_address())
            .and_then(|_| self.validate_signature())
            .and_then(|_| self.validate_timestamp())
    }

    pub fn validate_treshold_signature_proof(&mut self) -> Result<(), TxnValidatorError> {

        if ! self.txn.txn_signature.is_empty() {
            Ok(())
        } else {
            Err(TxnValidatorError::TxnSignatureTresholdIncorrect)
        }
    }

    pub fn validate(&mut self) -> Result<(), TxnValidatorError> {

        self.validate_structure()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().as_bytes().to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> TxnValidator {
        serde_json::from_slice::<TxnValidator>(data).unwrap()
    }    
}
