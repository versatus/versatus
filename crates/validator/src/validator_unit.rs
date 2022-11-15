use std::{
    collections::HashSet,
    sync::mpsc::{channel, Receiver, RecvError, SendError, Sender},
    thread::{self, *},
};

use left_right::{ReadHandle, ReadHandleFactory};
use mempool::mempool::{FetchFiltered, *};
use patriecia::{db::Database, inner::InnerTrie};
use txn::txn::Transaction;

use crate::{mempool_processor::MempoolControlMsg, txn_validator::TxnValidator};

/// Enum containing all messages related to controling the Core thread's
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreControlMsg {
    NewToProcess(u32),
    Stop,
    Start,
}

/// Custom type to represent Core Id
pub type CoreId = u8;

/// Struct containing all variables relevant for controlling and managing the
/// Core Join handle holds the thread responsible for validating the messages
/// State holds current CoreState
/// Control sender is channel ready to receive CoreControlMsg allowing control
/// over the thread state

#[derive(Debug)]
pub struct Core {
    pub join_handle: Option<JoinHandle<()>>,
    pub control_sender: Option<Sender<CoreControlMsg>>,
    error_sender: Sender<(CoreId, CoreError)>,
    pub id: CoreId,
}

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
    MempoolControlChannelUnreachable(SendError<MempoolControlMsg>),
}

fn send_core_err_msg(core_id: CoreId, error_sender: &Sender<(CoreId, CoreError)>, err: CoreError) {
    if let Err(err) = error_sender.send((core_id, err)) {
        // If failed to access error channel sth is really off
        // Log this
        _ = err;
    };
}
impl Core {
    /// Create new core with provided CoreId.
    ///
    /// Start function is required to spawn thread and create both join handle
    /// and assign control_handler.
    ///
    /// Arguments:
    ///
    /// * `id` ID to assign to that core
    /// * `error_sender` Sender end of the channel, used by the core to
    ///   propagate it's errors to main thread
    pub fn new<D: Database>(id: CoreId, error_sender: Sender<(CoreId, CoreError)>) -> Self {
        Self {
            join_handle: None,
            control_sender: None,
            error_sender,
            id,
        }
    }

    /// Start the core, runinng new thread, assigning control receiver
    ///
    /// Arguments:
    ///
    /// * `control_receiver` Recieving end of a channel on which the core will
    ///   be receiving ControlMsgs
    /// * `mempool_processor_sender` Sender of a channel, used to send control
    ///   messages with validated txns
    /// * `mempool_read_handle` Read handle to mempool containing pending
    ///   transactions
    /// * `amount_of_cores` Amount of cores to spawn in that validator core
    pub fn start<D: Database + 'static>(
        &mut self,
        control_receiver: Receiver<CoreControlMsg>,
        mempool_processor_sender: &Sender<MempoolControlMsg>,
        mempool_read_handle: &ReadHandle<Mempool>,
        state_read_handle_factory: &ReadHandleFactory<InnerTrie<D>>,
        amount_of_cores: u8,
    ) {
        // Cannot move self to thread so we need to copy values that will be used
        let mut state = CoreState::Ready;
        let id = self.id;
        let mempool_read_handle = mempool_read_handle.clone();
        let mempool_processor_sender = mempool_processor_sender.clone();
        let error_sender = self.error_sender.clone();

        let txn_validator = TxnValidator::new(state_read_handle_factory.handle());
        // Spawnin the core's thread
        let join_handle = thread::spawn(move || {
            // Entering the main state loop
            loop {
                match state {
                    // When core is ready for new txns to validate
                    CoreState::Ready => {
                        // Fetch msg. If msg::stop - stop and wait for resume
                        // if msg::NewToProcess - start processing the batch
                        // Blocking since we wait for next instruction on that channel
                        match control_receiver.recv() {
                            Ok(msg) => match msg {
                                CoreControlMsg::Stop => state = CoreState::Inactive,
                                CoreControlMsg::NewToProcess(amount) => {
                                    state = CoreState::Processing(amount)
                                },
                                _ => {
                                    // Propagating the error to core error receiver
                                    send_core_err_msg(
                                        id,
                                        &error_sender,
                                        CoreError::InvalidMsgForCurrentState(msg, state.clone()),
                                    );
                                    // Any error in core will result in it turning into inactive
                                    state = CoreState::Inactive;
                                },
                            },
                            Err(err) => {
                                // This should never happen, unless somehow the channels is dropped
                                // Since it can't be moved out of core struct, that'd mean that the
                                // whole struct is dropped
                                // That though means, that it'd be moved out of the ValidatorUnit
                                // struct, meaning that the whole
                                // validator unit has been dropped
                                send_core_err_msg(
                                    id,
                                    &error_sender,
                                    CoreError::FailedToReadFromControlChannel(err),
                                );
                                // Error = State::Inactive
                                state = CoreState::Inactive
                            },
                        }
                    },
                    CoreState::Inactive => {
                        // wait for activation
                        // Using blocking channel recv to idle while inactive
                        match control_receiver.recv() {
                            Ok(msg) => match msg {
                                CoreControlMsg::Start => state = CoreState::Ready,
                                _ => {
                                    send_core_err_msg(
                                        id,
                                        &error_sender,
                                        CoreError::InvalidMsgForCurrentState(msg, state.clone()),
                                    );

                                    // No need to set the state here, as the
                                    // core is already stopped
                                },
                            },
                            Err(err) => {
                                send_core_err_msg(
                                    id,
                                    &error_sender,
                                    CoreError::FailedToReadFromControlChannel(err),
                                );
                            },
                        }
                    },
                    CoreState::Processing(amount) => {
                        // Get batch of `amount` txns matching this core.id
                        let batch: Vec<TxnRecord> = mempool_read_handle
                            .fetch_pending(amount, |_, v| {
                                v.txn_id.as_bytes()[0] % amount_of_cores == id
                            });
                        let mut validated = HashSet::<(Transaction, bool)>::new();

                        // Group the txns by their validity
                        for txn_record in batch {
                            let txn = Transaction::from_string(&txn_record.txn);
                            match txn_validator.validate(&txn) {
                                Ok(_) => {
                                    validated.insert((txn, true));
                                },
                                Err(_) => {
                                    validated.insert((txn, false));
                                },
                            }
                        }

                        // Failure in sending validated txns to mempool processor will result in
                        // core going inactive That means though that the
                        // channel has been closed, meaning that mempool_processor is down
                        if let Err(err) = mempool_processor_sender
                            .send(MempoolControlMsg::NewValidated(validated))
                        {
                            send_core_err_msg(
                                id,
                                &error_sender,
                                CoreError::MempoolControlChannelUnreachable(err),
                            );
                            state = CoreState::Inactive;
                        } else {
                            // Finished processing, ready for new batch
                            state = CoreState::Ready;
                        }
                    },
                }
            }
        });

        // Spawned the thread, time to update the join handle
        self.join_handle = Some(join_handle);
    }
}
pub struct ValidatorUnit<D: Database> {
    pub cores: Vec<Core>,
    pub mempool_read_handle: ReadHandle<Mempool>,
    state_trie_read_handle_factory: ReadHandleFactory<InnerTrie<D>>,
    pub mempool_processor_sender: Sender<MempoolControlMsg>,
}

impl<D: Database + 'static> ValidatorUnit<D> {
    /// Create new instance of ValidatorUnit
    ///
    /// Arguments:
    ///
    /// * `mempool_read_handle` Read handle to the mempool with txns to validate
    /// * `state_trie_read_handle` Read handle to the state trie used to check
    ///   txns for their validity
    /// * `mempool_processor_sender` Sender end of a channel used by cores to
    ///   send validated messages
    /// * `amount_of_cores` An amount of cores the validator should be ready to
    ///   spawn
    /// * `core_error_channel_sender` Sender end of a channel used to propagate
    ///   cores' errors
    pub fn new(
        mempool_read_handle: ReadHandle<Mempool>,
        state_trie_read_handle_factory: ReadHandleFactory<InnerTrie<D>>,
        mempool_processor_sender: Sender<MempoolControlMsg>,
        amount_of_cores: u8,
        core_error_channel_sender: Sender<(CoreId, CoreError)>,
    ) -> Self {
        let mut cores = vec![];

        for i in 0..amount_of_cores {
            // Spawning new cores. Each core is a thread, that is pulling txns that match
            // it's self_id from mempool, validating them, and then finally
            // sending them to mempool processor for write. All errors from all
            // cores are propagated to core_error_channel, with error and core id of the
            // core that failed
            cores.push(Core::new::<D>(i, core_error_channel_sender.clone()));
        }
        Self {
            cores,
            mempool_read_handle,
            state_trie_read_handle_factory,
            mempool_processor_sender,
        }
    }

    /// Spawn `self.amount_of_cores` number of cores. Each core is it's separate
    /// thread.
    pub fn start(&mut self) {
        let amount_of_cores = self.cores.len() as u8;
        for core in &mut self.cores {
            let (control_sender, control_receiver) = channel();
            core.control_sender = Some(control_sender);
            core.start(
                control_receiver,
                &self.mempool_processor_sender,
                &self.mempool_read_handle,
                &self.state_trie_read_handle_factory,
                amount_of_cores,
            );
        }
    }
}
