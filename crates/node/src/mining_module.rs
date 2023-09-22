use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use block::Block;
use bulldag::graph::BullDag;
use events::{Event, EventMessage, EventPublisher};
use mempool::MempoolReadHandleFactory;
use miner::Miner;
use primitives::Address;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler};
use vrrb_config::NodeConfig;
use vrrb_core::transactions::TransactionKind;

use crate::{NodeError, RuntimeComponent, RuntimeComponentHandle};

#[derive(Debug, Clone)]
pub struct MiningModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    miner: Miner,
}

#[derive(Debug, Clone)]
pub struct MiningModuleConfig {
    pub miner: Miner,
}

impl MiningModule {
    pub fn new(cfg: MiningModuleConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            label: String::from("Miner"),
            status: ActorState::Stopped,
            miner: cfg.miner,
        }
    }
}
impl MiningModule {
    // fn _take_snapshot_until_cutoff(&self, cutoff_idx: usize) -> Vec<TransactionKind> {
    //     let mut handle = self._mempool_read_handle_factory.handle();
    //
    //     // TODO: drain mempool instead then commit changes
    //     handle
    //         .drain(..cutoff_idx)
    //         .map(|(_id, record)| record.txn)
    //         .collect()
    // }
    //
    // fn _mark_snapshot_transactions(&mut self, cutoff_idx: usize) {
    //     telemetry::info!("Marking transactions as mined until index: {}", cutoff_idx);
    //     // TODO: run a batch update to mark txns as being processed
    // }
}

#[async_trait]
impl Handler<EventMessage> for MiningModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        format!("Miner::{}", self.id())
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_start(&self) {
        info!("{} starting", self.label());
    }

    fn on_stop(&self) {
        info!("{} received stop signal. Stopping", self.label(),);
    }

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },
            // Event::ElectedMiner((_winner_claim_hash, winner_claim)) => {
            //     if self.miner.check_claim(winner_claim.hash) {
            //         let mining_result = self.miner.try_mine();
            //
            //         if let Ok(block) = mining_result {
            //             let _ = self
            //                 .events_tx
            //                 .send(Event::MinedBlock(block.clone()).into())
            //                 .await
            //                 .map_err(|err| {
            //                     theater::TheaterError::Other(format!(
            //                         "failed to send mined block to event bus: {err}"
            //                     ))
            //                 });
            //         }
            //     };
            // },
            // Event::CheckConflictResolution((proposal_blocks, round, seed, convergence_block)) =>
            // {     let tmp_proposal_blocks = proposal_blocks.clone();
            //     let resolved_proposals_set = self
            //         .miner
            //         .resolve(&tmp_proposal_blocks, round, seed)
            //         .iter()
            //         .cloned()
            //         .collect::<HashSet<ProposalBlock>>();
            //
            //     let proposal_blocks_set = proposal_blocks
            //         .iter()
            //         .cloned()
            //         .collect::<HashSet<ProposalBlock>>();
            //
            //     if proposal_blocks_set == resolved_proposals_set {
            //         if let Err(err) = self
            //             .events_tx
            //             .send(EventMessage::new(
            //                 None,
            //                 Event::SignConvergenceBlock(convergence_block),
            //             ))
            //             .await
            //         {
            //             theater::TheaterError::Other(format!(
            //                 "failed to send EventMessage for Event::SignConvergenceBlock: {err}"
            //             ));
            //         };
            //     }
            // },
            Event::NoOp => {},
            _ => {},
        }
        Ok(ActorState::Running)
    }
}

// TODO: figure out how to avoid this
unsafe impl Send for MiningModule {}
