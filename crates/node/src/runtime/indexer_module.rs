use std::{hash::Hash, path::PathBuf};

use async_trait::async_trait;
use events::{DirectedEvent, Event, Topic};
use lr_trie::ReadHandleFactory;
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use patriecia::{db::MemoryDB, inner::InnerTrie};
use storage::vrrbdb::{VrrbDb, VrrbDbReadHandle};
use telemetry::{info, warn};
use theater::{Actor, ActorId, ActorLabel, ActorState, Handler, Message, TheaterError};
use tokio::sync::broadcast::error::TryRecvError;
use vrrb_core::txn::{TransactionDigest, Txn};
use vrrb_http::indexer::{IndexerClient, IndexerClientConfig};

use crate::{
    result::Result,
    EventBroadcastSender,
    NodeError,
    RuntimeModule,
    MEMPOOL_THRESHOLD_SIZE,
};

pub struct IndexerModuleConfig {
    pub mempool: LeftRightMempool,
    pub events_tx: EventBroadcastSender,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
}

#[derive(Debug)]
pub struct IndexerModule {
    // mempool: LeftRightMempool,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    // events_tx: EventBroadcastSender,
    cutoff_transaction: Option<TransactionDigest>,
    indexer_client: IndexerClient,
    mempool_read_handle_factory: MempoolReadHandleFactory,
}

impl IndexerModule {
    pub fn new(config: IndexerModuleConfig) -> Self {
        let indexer_config = IndexerClientConfig::default();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            // mempool: config.mempool,
            // events_tx: config.events_tx,
            status: ActorState::Stopped,
            label: String::from("Indexer"),
            cutoff_transaction: None,
            indexer_client: IndexerClient::new(indexer_config).unwrap(),
            mempool_read_handle_factory: config.mempool_read_handle_factory,
        }
    }
}

impl IndexerModule {}

#[async_trait]
impl Handler<Event> for IndexerModule {
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

            Event::NewTxnCreated(txn) => {
                info!("Sending transaction to indexer: NewTxnCreated");

                let txn_records = self.mempool_read_handle_factory.entries().clone();
                let txn_record = txn_records.get(&txn.id());

                let client = self.indexer_client.clone();
                match client.post_tx(txn_record.unwrap()).await {
                    Ok(_) => {
                        info!("Successfully sent TxnRecord to indexer");
                    },
                    Err(e) => {
                        warn!("Error sending TxnRecord to indexer {}", e);
                    },
                }
            },

            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
