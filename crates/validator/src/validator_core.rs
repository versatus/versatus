use std::{
    collections::HashSet,
    sync::mpsc::{channel, Receiver, RecvError, SendError, Sender},
    thread::{self, *},
};

use left_right::{ReadHandle, ReadHandleFactory};
use mempool::mempool::{FetchFiltered, *};
use patriecia::{db::Database, inner::InnerTrie};
use vrrb_core::txn::*;

use crate::txn_validator::{StateSnapshot, TxnValidator};

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
    validator: TxnValidator,
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
    pub fn new(id: CoreId, validator: TxnValidator) -> Self {
        Self { id, validator }
    }

    pub fn id(&self) -> CoreId {
        self.id.clone()
    }

    pub fn process_transactions(
        &self,
        state_snapshot: &StateSnapshot,
        batch: Vec<Txn>,
    ) -> HashSet<(Txn, crate::txn_validator::Result<()>)> {
        // ) -> HashSet<(Txn, bool)> {
        batch
            .into_iter()
            .map(|txn| match self.validator.validate(state_snapshot, &txn) {
                Ok(_) => (txn, Ok(())),
                Err(err) => {
                    telemetry::error!("{err:?}");

                    (txn, Err(err))
                    // Should we send error?
                    // send_core_err_msg(id, &error_sender,
                    // err);
                },
            })
            .collect::<HashSet<(Txn, crate::txn_validator::Result<()>)>>()
    }
}
