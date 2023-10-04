use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use primitives::{Address, PublicKey, SecretKey, Signature};
use crate::transactions::{Token, Transaction, TransactionDigest, Transfer, TransferBuilder, TxAmount, TxNonce, TxTimestamp};


#[derive(Hash, Debug, Deserialize, Clone, Serialize, Eq, PartialEq)]
pub enum TransactionKind {
    Transfer(Transfer),
}

impl TransactionKind {
    pub fn transfer_builder() -> TransferBuilder {
        Transfer::builder()
    }
}

impl Default for TransactionKind {
    fn default() -> Self {
        TransactionKind::Transfer(Transfer::default())
    }
}

impl Transaction for TransactionKind {
    fn id(&self) -> TransactionDigest {
        match self {
            TransactionKind::Transfer(transfer) => transfer.id(),
        }
    }

    fn timestamp(&self) -> TxTimestamp {
        match self {
            TransactionKind::Transfer(transfer) => transfer.timestamp(),
        }
    }

    fn sender_address(&self) -> Address {
        match self {
            TransactionKind::Transfer(transfer) => transfer.sender_address(),
        }
    }

    fn sender_public_key(&self) -> PublicKey {
        match self {
            TransactionKind::Transfer(transfer) => transfer.sender_public_key(),
        }
    }

    fn receiver_address(&self) -> Address {
        match self {
            TransactionKind::Transfer(transfer) => transfer.receiver_address(),
        }
    }

    fn token(&self) -> Token {
        match self {
            TransactionKind::Transfer(transfer) => transfer.token(),
        }
    }

    fn amount(&self) -> TxAmount {
        match self {
            TransactionKind::Transfer(transfer) => transfer.amount(),
        }
    }

    fn signature(&self) -> Signature {
        match self {
            TransactionKind::Transfer(transfer) => transfer.signature(),
        }
    }

    fn validators(&self) -> Option<HashMap<String, bool>> {
        match self {
            TransactionKind::Transfer(transfer) => transfer.validators(),
        }
    }

    fn nonce(&self) -> TxNonce {
        match self {
            TransactionKind::Transfer(transfer) => transfer.nonce(),
        }
    }

    fn fee(&self) -> u128 {
        match self {
            TransactionKind::Transfer(transfer) => transfer.fee(),
        }
    }

    fn validator_fee_share(&self) -> u128 {
        match self {
            TransactionKind::Transfer(transfer) => transfer.validator_fee_share(),
        }
    }

    fn proposer_fee_share(&self) -> u128 {
        match self {
            TransactionKind::Transfer(transfer) => transfer.proposer_fee_share(),
        }
    }

    fn build_payload(&self) -> String {
        match self {
            TransactionKind::Transfer(transfer) => transfer.build_payload(),
        }
    }

    fn digest(&self) -> TransactionDigest {
        match self {
            TransactionKind::Transfer(transfer) => transfer.digest(),
        }
    }

    fn sign(&mut self, sk: &SecretKey) {
        match self {
            TransactionKind::Transfer(transfer) => transfer.sign(sk),
        }
    }
}
