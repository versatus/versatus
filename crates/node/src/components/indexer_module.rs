use async_trait::async_trait;
use events::{Event, EventMessage, EventSubscriber};
use mempool::MempoolReadHandleFactory;
use telemetry::{info, warn};
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler};
use tokio::task::JoinHandle;
use vrrb_config::NodeConfig;
use vrrb_http::indexer::{IndexerClient, IndexerClientConfig};

use crate::{NodeError, Result};

pub struct IndexerModuleConfig {
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
}

#[derive(Debug)]
pub struct IndexerModule {
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    indexer_client: IndexerClient,
    mempool_read_handle_factory: MempoolReadHandleFactory,
}

impl IndexerModule {
    pub fn new(config: IndexerModuleConfig) -> Self {
        let indexer_config = IndexerClientConfig::default();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ActorState::Stopped,
            label: String::from("Indexer"),
            indexer_client: IndexerClient::new(indexer_config).unwrap(),
            mempool_read_handle_factory: config.mempool_read_handle_factory,
        }
    }
}

impl IndexerModule {}

#[async_trait]
impl Handler<EventMessage> for IndexerModule {
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

    async fn handle(&mut self, event: EventMessage) -> theater::Result<ActorState> {
        match event.into() {
            Event::Stop => {
                return Ok(ActorState::Stopped);
            },

            Event::TxnAddedToMempool(transaction_digest) => {
                info!("Sending transaction to indexer: NewTxnCreated");

                let txn_records = self.mempool_read_handle_factory.entries();
                if let Some(txn_record) = txn_records.get(&transaction_digest) {
                    let client = self.indexer_client.clone();
                    match client.post_tx(txn_record).await {
                        Ok(_) => {
                            info!("Successfully sent TxnRecord to indexer");
                        },
                        Err(e) => {
                            warn!("Could not send TxnRecord to indexer {}", e);
                        },
                    }
                } else {
                    warn!("Transaction record not found in mempool");
                }
            },

            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}

pub fn setup_indexer_module(
    _config: &NodeConfig,
    mut indexer_events_rx: EventSubscriber,
    mempool_read_handle_factory: MempoolReadHandleFactory,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let config = IndexerModuleConfig {
        mempool_read_handle_factory,
    };

    let module = IndexerModule::new(config);

    let mut indexer_module_actor = ActorImpl::new(module);

    let indexer_handle = tokio::spawn(async move {
        indexer_module_actor
            .start(&mut indexer_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(indexer_handle))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use events::DEFAULT_BUFFER;
    use mempool::LeftRightMempool;
    use serial_test::serial;
    use theater::{Actor, ActorImpl};

    use super::*;

    // Helper function to create a test instance of IndexerModule
    fn create_test_indexer_module() -> IndexerModule {
        let mempool = Arc::new(LeftRightMempool::default());
        let mempool_read_handle_factory = mempool.factory();
        let config = IndexerModuleConfig {
            mempool_read_handle_factory,
        };

        IndexerModule::new(config)
    }

    #[test]
    fn test_new_indexer_module() {
        let indexer_module = create_test_indexer_module();

        assert_eq!(indexer_module.status, ActorState::Stopped);
        assert_eq!(indexer_module.label, "Indexer");
    }

    #[test]
    fn test_indexer_module_id_label_status() {
        let indexer_module = create_test_indexer_module();

        assert_eq!(indexer_module.label(), "Indexer");
        assert_eq!(indexer_module.status(), ActorState::Stopped);

        let id = indexer_module.id();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_indexer_module_start_and_stop() {
        let indexer_module = create_test_indexer_module();

        assert_eq!(indexer_module.label(), "Indexer");
        assert_eq!(indexer_module.status(), ActorState::Stopped);

        let (ctrl_tx, mut indexer_events_rx) = tokio::sync::broadcast::channel(DEFAULT_BUFFER);

        let mut indexer_module_actor = ActorImpl::new(indexer_module);

        let indexer_handle = tokio::spawn(async move {
            indexer_module_actor
                .start(&mut indexer_events_rx)
                .await
                .unwrap()
        });

        ctrl_tx.send(Event::Stop.into()).unwrap();
        indexer_handle.await.unwrap();
    }
}
