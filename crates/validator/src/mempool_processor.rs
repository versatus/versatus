use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
    sync::mpsc::{Receiver, RecvError, SendError, Sender},
};

use mempool::{
    error::MempoolError,
    mempool::{LeftRightMemPoolDB, TxnStatus},
};
use patriecia::db::Database;
use txn::txn::Transaction;

use crate::validator_unit::{Core, CoreControlMsg, CoreId, ValidatorUnit};

#[derive(Debug, PartialEq, Eq)]
pub enum MempoolTxnProcessorError {
    FailedToObtainControlChannel(CoreId),
    ControlSendError(CoreId, SendError<CoreControlMsg>),
    FailedToWriteToMempool(MempoolError),
    InvalidMsgForCurrentState(MempoolControlMsg, MempoolTxnProcessorState),
    FailedToReadFromControlChannel(RecvError),
}
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MempoolTxnProcessorState {
    Active,
    Inactive,
}
pub struct MempoolTxnProcessor<D>
where
    D: Database,
{
    error_sender: Sender<MempoolTxnProcessorError>,
    control_receiver: Receiver<MempoolControlMsg>,
    pub validator: ValidatorUnit<D>,
    mempool: LeftRightMemPoolDB,
}

impl<D> MempoolTxnProcessor<D>
where
    D: Database + 'static,
{
    pub fn new(
        control_receiver: Receiver<MempoolControlMsg>,
        validator: ValidatorUnit<D>,
        mempool: LeftRightMemPoolDB,
        error_sender: Sender<MempoolTxnProcessorError>,
    ) -> Self {
        Self {
            control_receiver,
            validator,
            mempool,
            error_sender,
        }
    }

    fn send_err_msg(&self, err: MempoolTxnProcessorError) {
        if let Err(err) = self.error_sender.send(err) {
            // The only reason this would happen is that user intentionally dropped error
            // channel. Meaning that they don't care about the error.
            // TODO: Maybe there's better way to do that
            _ = err;
        };
    }

    fn send_core_msg(&self, core: &Core, msg: CoreControlMsg) {
        match core.control_sender.as_ref() {
            Some(sender) => {
                if let Err(err) = sender.send(msg) {
                    self.send_err_msg(MempoolTxnProcessorError::ControlSendError(core.id, err))
                }
            },
            None => {
                self.send_err_msg(MempoolTxnProcessorError::FailedToObtainControlChannel(
                    core.id,
                ));
            },
        }
    }

    pub fn start(&mut self) {
        // Start the validator, spawning `validator.amount_of_cores` number of cores
        self.validator.start();
        // Start with state = Active
        let mut state = MempoolTxnProcessorState::Active;

        // Main state machine loop
        loop {
            match state {
                MempoolTxnProcessorState::Active => {
                    // Wait for next msg from control channel
                    match self.control_receiver.recv() {
                        Ok(msg) => {
                            match msg {
                                // if mempool processor is to be stopped, all cores should be
                                // stopped too
                                MempoolControlMsg::Stop => {
                                    for core in &self.validator.cores {
                                        self.send_core_msg(core, CoreControlMsg::Stop);
                                    }
                                    state = MempoolTxnProcessorState::Inactive;
                                },
                                // Upon receiving new txns from the network over the control channel
                                // Those should be added to pending mempool
                                MempoolControlMsg::NewFromNetwork(txns) => {
                                    let amount_of_cores = self.validator.cores.len();
                                    let mut amount_of_txns = vec![0; amount_of_cores];
                                    for txn in &txns {
                                        let mut hash = DefaultHasher::new();
                                        txn.hash(&mut hash);
                                        amount_of_txns[(hash.finish() as u8 % amount_of_cores as u8)
                                            as usize] += 1;
                                    }

                                    // This mempool function always return OK
                                    // Implemented error handling just in case it's changed in the
                                    // future
                                    if let Err(err) =
                                        self.mempool.add_txn_batch(&txns, TxnStatus::Pending)
                                    {
                                        if let Err(err) = self.error_sender.send(
                                            MempoolTxnProcessorError::FailedToWriteToMempool(err),
                                        ) {
                                            // This can happen only if user decides to drop error
                                            // channel receiver, meaning that they want the errors
                                            // supressed
                                            _ = err;
                                        }
                                    }

                                    // Send messages to all cores, with amount of transactions that
                                    // are theirs to validate
                                    for core in &self.validator.cores {
                                        self.send_core_msg(
                                            core,
                                            CoreControlMsg::NewToProcess(
                                                // This index is always in range, as core_id ∈ [0,
                                                // amount_of_cores)
                                                amount_of_txns[core.id as usize],
                                            ),
                                        );
                                    }
                                },

                                // When validated txns come, add them to validated mempool
                                MempoolControlMsg::NewValidated(txns) => {
                                    let mut txns_valid = HashSet::new();
                                    let mut txns_invalid = HashSet::new();
                                    let mut txns_to_remove = HashSet::new();
                                    txns.iter().for_each(|(txn, valid)| {
                                        if *valid {
                                            txns_valid.insert(txn.clone());
                                        } else {
                                            txns_invalid.insert(txn.clone());
                                        }
                                        txns_to_remove.insert(txn.clone());
                                    });

                                    // Write valid txns to proper place in mempool
                                    // Again, for now this function does not throw any errors,
                                    // always returning Ok(())
                                    if let Err(err) = self
                                        .mempool
                                        .add_txn_batch(&txns_valid, TxnStatus::Validated)
                                    {
                                        self.send_err_msg(
                                            MempoolTxnProcessorError::FailedToWriteToMempool(err),
                                        );
                                    };

                                    // Write invalid txns to proper place in mempool
                                    if let Err(err) = self
                                        .mempool
                                        .add_txn_batch(&txns_invalid, TxnStatus::Rejected)
                                    {
                                        self.send_err_msg(
                                            MempoolTxnProcessorError::FailedToWriteToMempool(err),
                                        );
                                    };
                                    if let Err(err) = self
                                        .mempool
                                        .remove_txn_batch(&txns_to_remove, TxnStatus::Pending)
                                    {
                                        self.send_err_msg(
                                            MempoolTxnProcessorError::FailedToWriteToMempool(err),
                                        );
                                    };
                                },
                                _ => self.send_err_msg(
                                    MempoolTxnProcessorError::InvalidMsgForCurrentState(
                                        msg,
                                        state.clone(),
                                    ),
                                ),
                            }
                        },
                        Err(err) => self.send_err_msg(
                            MempoolTxnProcessorError::FailedToReadFromControlChannel(err),
                        ),
                    }
                },

                // When inactive, wait for activation
                MempoolTxnProcessorState::Inactive => match self.control_receiver.recv() {
                    Ok(msg) => match msg {
                        MempoolControlMsg::Start => {
                            // When activated again, reactivate all cores too
                            for core in &self.validator.cores {
                                self.send_core_msg(core, CoreControlMsg::Start);
                            }
                            state = MempoolTxnProcessorState::Active;
                        },
                        _ => self.send_err_msg(
                            MempoolTxnProcessorError::InvalidMsgForCurrentState(msg, state.clone()),
                        ),
                    },
                    Err(err) => self.send_err_msg(
                        MempoolTxnProcessorError::FailedToReadFromControlChannel(err),
                    ),
                },
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MempoolControlMsg {
    NewFromNetwork(HashSet<Transaction>),
    NewValidated(HashSet<(Transaction, bool)>),
    Start,
    Stop,
}
