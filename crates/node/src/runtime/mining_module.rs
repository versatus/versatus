use async_trait::async_trait;
use block::{convergence_block, Block};
use events::{Event, EventMessage, EventPublisher};
use mempool::MempoolReadHandleFactory;
use miner::Miner;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{ActorId, ActorLabel, ActorState, Handler};
use tokio::sync::broadcast::{error::TryRecvError, Receiver};
use vrrb_core::txn::Txn;

#[derive(Debug)]
pub struct MiningModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
    miner: Miner,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
}

#[derive(Debug, Clone)]
pub struct MiningModuleConfig {
    pub events_tx: EventPublisher,
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
        telemetry::info!("Marking transactions as mined until index: {}", cutoff_idx);
        // TODO: run a batch update to mark txns as being processed
    }
}

#[async_trait]
impl Handler<EventMessage> for MiningModule {
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

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            Event::ElectedMiner((winner_claim_hash, winner_claim)) => {
                if self.miner.check_claim(winner_claim.hash) {
                    let mining_result = self.miner.try_mine();

                    if let Ok(block) = mining_result {
                        let _ = self
                            .events_tx
                            .send(Event::MinedBlock(block.clone()).into())
                            .await
                            .map_err(|err| {
                                theater::TheaterError::Other(format!(
                                    "failed to send mined block to event bus: {err}"
                                ))
                            });
                    }
                };
            },
            Event::NoOp => {},
            _ => {},
        }
        Ok(ActorState::Running)
    }
}

// TODO: figure out how to avoid this
unsafe impl Send for MiningModule {}
