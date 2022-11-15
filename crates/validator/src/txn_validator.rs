use std::{
    result::Result as StdResult,
    time::{SystemTime, UNIX_EPOCH},
};

use left_right::ReadHandle;
use lr_trie::{GetDeserialized, LeftRightTrieError};
use lrdb::Account;
use patriecia::{db::Database, error::TrieError, inner::InnerTrie};
use primitives::types::PublicKey;
#[allow(deprecated)]
use txn::txn::{CallData, Code, SystemInstruction, Transaction, Txn};

type Result<T> = StdResult<T, TxnValidatorError>;

pub const ADDRESS_PREFIX: &str = "0x192";
pub enum TxnFees {
    Slow,
    Fast,
    Instant,
}

#[derive(PartialEq, Eq, Debug)]
pub enum TxnValidatorError {
    InvalidInstruction,
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

    pub fn validate_signature(&self, txn: &Transaction) -> Result<()> {
        txn.verify_signature()
            .map_err(|_| TxnValidatorError::TxnSignatureIncorrect)
    }

    /// Txn receiver validator
    // TODO, to be synchronized with Wallet.
    // TODO: ADDRESS_PREFIX should only be verified on testnet, as it seems the prefix
    // is only used there
    #[deprecated = "Replaced with instruction validation"]
    #[allow(deprecated)]
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
    /// TODO: The time should be validated by block_height or blockhash,
    /// any kind of time that the network has consensus on
    /// systemtime may be problematic in p2p network
    #[allow(deprecated)]
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

    fn validate_single_instruction(
        &self,
        ix: &SystemInstruction,
        signer: &PublicKey,
    ) -> Result<()> {
        match ix {
            SystemInstruction::Transfer(transfer_data) => {
                if transfer_data.from != *signer {
                    return Err(TxnValidatorError::TxnSignatureIncorrect);
                }

                let data: StdResult<Account, LeftRightTrieError> = self
                    .state
                    .get_deserialized_data(transfer_data.from.clone().serialize().to_vec());
                match data {
                    Ok(account) => {
                        if (account.credits - account.debits)
                            //TODO: verify the correct token unit is used here
                            .checked_sub(transfer_data.amount.0)
                            .is_none()
                        {
                            return Err(TxnValidatorError::TxnAmountIncorrect);
                        };
                        Ok(())
                    },
                    Err(_) => Err(TxnValidatorError::InvalidSender),
                }
            },
            SystemInstruction::ContractDeploy(code) => self.validate_contract_deploy(code),
            SystemInstruction::ContractUpgrade(code) => self.validate_contract_upgrade(code),
            SystemInstruction::ContractCall(call_data) => self.validate_contract_call(call_data),
            _ => Err(TxnValidatorError::InvalidInstruction),
        }
    }

    // TODO: Implement this once we have vm and contracts
    fn validate_contract_call(&self, _call_data: &CallData) -> Result<()> {
        Ok(())
    }

    // TODO: Implement this once we have vm and contracts
    fn validate_contract_upgrade(&self, _code: &Code) -> Result<()> {
        Ok(())
    }

    // TODO: Implement this once we have vm and contracts
    fn validate_contract_deploy(&self, _code: &Code) -> Result<()> {
        Ok(())
    }

    fn validate_instructions(&self, txn: &Transaction) -> Result<()> {
        self.validate_signature(txn)?;
        let signer = txn.sender;
        for ix in &txn.instructions {
            self.validate_single_instruction(ix, &signer)?;
        }
        Ok(())
    }

    /// An entire Txn validator
    // TODO: include fees and signature threshold.
    pub fn validate(&self, txn: &Transaction) -> Result<()> {
        self.validate_signature(txn)?;
        self.validate_instructions(txn)?;

        Ok(())
    }
}
