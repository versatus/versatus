use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::RecvError,
};

use mempool::MempoolReadHandleFactory;
use primitives::Address;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use storage::vrrbdb::{StateStoreReadHandleFactory, ClaimStoreReadHandleFactory};
use vrrb_core::{account::Account, claim::Claim, transactions::TransactionDigest};
use vrrb_core::transactions::TransactionKind;

use crate::{claim_validator::ClaimValidator, txn_validator::TxnValidator};

/// Enum containing all messages related to controling the Core thread's
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreControlMsg {
    NewToProcess(u32),
    Stop,
    Start,
}

/// Custom type to represent Core Id
pub type CoreId = u8;

/// Enum of all possible CoreStates
/// Ready meaning the core is ready for new txns to process
/// Inactive means that either error happened or the core was stopped
/// Processing means that the core received a batch to validate and is working
/// on it
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum CoreState {
    Ready,
    Inactive,
    Processing(u32),
}
#[derive(Debug, PartialEq, Eq)]
pub enum CoreError {
    InvalidMsgForCurrentState(CoreControlMsg, CoreState),
    FailedToReadFromControlChannel(RecvError),
}

/// Struct containing all variables relevant for controlling and managing the
/// Core Join handle holds the thread responsible for validating the messages
/// State holds current CoreState
/// Control sender is channel ready to receive CoreControlMsg allowing control
/// over the thread state
#[derive(Debug, Clone)]
pub struct Core {
    id: CoreId,
    txn_validator: TxnValidator,
    claims_validator: ClaimValidator,
}

impl Core {
    /// Create new core with provided CoreId.
    ///
    /// Arguments:
    ///
    /// * `id` ID to assign to that core
    /// * `error_sender` Sender end of the channel, used by the core to
    ///   propagate it's errors to main thread
    // pub fn new<D: Database>(id: CoreId, error_sender: Sender<(CoreId,
    // CoreError)>) -> Self {
    pub fn new(
        id: CoreId, 
        txn_validator: TxnValidator, 
        claims_validator: ClaimValidator,
    ) -> Self {
        Self {
            id,
            txn_validator,
            claims_validator,
        }
    }

    pub fn id(&self) -> CoreId {
        self.id
    }

    pub fn process_transaction_kind(
        &self,
        transaction: &TransactionDigest,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory
    ) -> crate::txn_validator::Result<TransactionKind> {
        if let Some(txn) = mempool_reader.handle().get(transaction) {
            self.txn_validator.validate(state_reader, &txn.txn)?;
            return Ok(txn.txn.clone())
        }

        return Err(crate::txn_validator::TxnValidatorError::NotFound)
    }

    pub fn process_transactions(
        &self,
        batch: Vec<TransactionKind>,
        mempool_reader: MempoolReadHandleFactory,
        state_reader: StateStoreReadHandleFactory,
    ) -> HashSet<(TransactionKind, crate::txn_validator::Result<()>)> {
        batch
            .into_iter()
            .map(
                |txn| match self.txn_validator.validate(state_reader.clone(), &txn) {
                    Ok(_) => (txn, Ok(())),
                    Err(err) => {
                        telemetry::error!("{err:?}");
                        (txn, Err(err))
                    },
                },
            )
            .collect::<HashSet<(TransactionKind, crate::txn_validator::Result<()>)>>()
    }

    /// The function processes a batch of claims parallely using a claims
    /// validator and returns a set of tuples containing the claim and the
    /// result of the validation.
    ///
    /// Arguments:
    ///
    /// * `batch`: A vector of `Claim` objects that need to be processed
    ///   parallely.
    ///
    /// Returns:
    ///
    /// The function `process_claims` returns a `HashSet` containing tuples of
    /// `(Claim, Result<(), ClaimValidationError>)`. Each tuple represents a
    /// claim from the input `batch` and the result of validating that claim
    /// using the `claims_validator` field of the struct. If the validation is
    /// successful, the result is `Ok(())`, otherwise it is an `Err` containing
    /// a `ClaimValidationError`.
    pub fn process_claims(
        &self,
        batch: Vec<Claim>,
    ) -> HashSet<(Claim, crate::claim_validator::Result<()>)> {
        batch
            .par_iter()
            .map(|claim| match self.claims_validator.validate(claim) {
                Ok(_) => (claim.clone(), Ok(())),
                Err(err) => {
                    telemetry::error!("{err:?}");
                    (claim.clone(), Err(err))
                },
            })
            .collect::<HashSet<(Claim, crate::claim_validator::Result<()>)>>()
    }
}
