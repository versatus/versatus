use async_trait::async_trait;
use block::{Block, convergence_block};
use events::{DirectedEvent, Event};
use mempool::MempoolReadHandleFactory;
use miner::Miner;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tokio::sync::broadcast::{error::TryRecvError, Receiver};

use vrrb_core::txn::Txn;
use crate::EventBroadcastSender;


#[derive(Debug)]
pub struct MiningModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventBroadcastSender,
    miner: Miner,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
}

#[derive(Debug, Clone)]
pub struct MiningModuleConfig {
    pub events_tx: EventBroadcastSender,
    pub miner: Miner,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
}

impl MiningModule {
    pub fn new(cfg: MiningModuleConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("Miner"),
            status: ActorState::Stopped,
            events_tx: cfg.events_tx,
            miner: cfg.miner,
            vrrbdb_read_handle: cfg.vrrbdb_read_handle,
            mempool_read_handle_factory: cfg.mempool_read_handle_factory,
        }
    }
}
impl MiningModule {
    // fn take_snapshot_until_cutoff(&self, cutoff_idx: usize) -> Vec<Txn> {
    fn take_snapshot_until_cutoff(&self, cutoff_idx: usize) -> Vec<Txn> {
        let mut handle = self.mempool_read_handle_factory.handle();

        // TODO: drain mempool instead then commit changes
        handle
            .drain(..cutoff_idx)
            .map(|(id, record)| {
                dbg!(id);
                record.txn
            })
            .collect()
    }

    fn mark_snapshot_transactions(&mut self, cutoff_idx: usize) {
        info!("Marking transactions as mined until index: {}", cutoff_idx);
    }
}

#[async_trait]
impl Handler<Event> for MiningModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.label.clone()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{}-{} starting", self.label(), self.id(),);
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.label(),
            self.id(),
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },

            Event::ElectedMiner((winner_claim_hash, winner_claim)) => {
                if self.miner.check_claim(winner_claim.hash) {
                    let mining_result = self.miner.try_mine();

                    if let Ok(block) = mining_result {
                        let _ = self.events_tx.send(
                            Event::MinedBlock(block.clone())
                        );
                    }
                };
            },
            Event::TxnAddedToMempool(_) => {
                // dbg!(txn_digest.to_string());
            },
            Event::MempoolSizeThesholdReached { cutoff_transaction } => {
                let handle = self.mempool_read_handle_factory.handle();

                if let Some(idx) = handle.get_index_of(&cutoff_transaction) {
                    dbg!(handle.len());
                    let transaction_snapshot = self.take_snapshot_until_cutoff(idx);
                    dbg!(transaction_snapshot.len());
                    dbg!(handle.len());

                    self.mark_snapshot_transactions(idx);
                } else {
                    telemetry::error!(
                        "Could not find index of cutoff transaction to produce a block"
                    );
                }
            },
            Event::BlockConfirmed(_) => {
                // do something
            },
            Event::NoOp => {},
            // _ => telemetry::warn!("unrecognized command received: {:?}", event),
            _ => {},
        }
        Ok(ActorState::Running)
    }
}

unsafe impl Sync for MiningModule {}
unsafe impl Send for MiningModule {}
