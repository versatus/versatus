use std::{hash::Hash, path::PathBuf};

use async_trait::async_trait;
use lr_trie::ReadHandleFactory;
use mempool::LeftRightMempool;
use patriecia::{db::MemoryDB, inner::InnerTrie};
use storage::vrrbdb::{VrrbDb, VrrbDbReadHandle};
use telemetry::info;
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::{
    event_router::{DirectedEvent, Event, Topic},
    txn::Txn,
};

use crate::{result::Result, NodeError, RuntimeModule};

pub struct MempoolModuleConfig {
    pub mempool: LeftRightMempool,
    pub events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

#[derive(Debug)]
pub struct MempoolModule {
    mempool: LeftRightMempool,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: tokio::sync::mpsc::UnboundedSender<DirectedEvent>,
}

impl MempoolModule {
    pub fn new(config: MempoolModuleConfig) -> Self {
        Self {
            mempool: config.mempool,
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            label: String::from("State"),
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

impl MempoolModule {
    fn name(&self) -> String {
        String::from("Mempool module")
    }
}

#[async_trait]
impl Handler<Event> for MempoolModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.name()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn set_status(&mut self, actor_status: ActorState) {
        self.status = actor_status;
    }

    fn on_stop(&self) {
        info!(
            "{}-{} received stop signal. Stopping",
            self.name(),
            self.label()
        );
    }

    async fn handle(&mut self, event: Event) -> theater::Result<ActorState> {
        match event {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },

            Event::BlockReceived => {},

            Event::NewTxnCreated(txn) => {
                info!("Storing transaction in mempool for validation");

                let txn_hash = txn.digest();

                self.mempool
                    .insert(txn)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send((Topic::Transactions, Event::TxnAddedToMempool(txn_hash)))
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::TxnValidated(txn) => {
                self.mempool
                    .remove(&txn.digest())
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
