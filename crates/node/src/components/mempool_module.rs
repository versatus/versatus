use async_trait::async_trait;
use events::{Event, EventMessage, EventPublisher, EventSubscriber};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use telemetry::info;
use theater::{Actor, ActorId, ActorImpl, ActorLabel, ActorState, Handler, TheaterError};
use vrrb_core::txn::TransactionDigest;

use crate::{
    NodeError,
    RuntimeComponent,
    RuntimeComponentHandle,
    RuntimeComponentHealthReport,
    RuntimeHandle,
    MEMPOOL_THRESHOLD_SIZE,
};

#[derive(Debug, Clone)]
pub struct MempoolModuleConfig {
    pub mempool: LeftRightMempool,
    pub events_tx: EventPublisher,
}

#[derive(Debug)]
pub struct MempoolModule {
    mempool: LeftRightMempool,
    status: ActorState,
    label: ActorLabel,
    id: ActorId,
    events_tx: EventPublisher,
    cutoff_transaction: Option<TransactionDigest>,
}

impl MempoolModule {
    pub fn new(config: MempoolModuleConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            mempool: config.mempool,
            events_tx: config.events_tx,
            status: ActorState::Stopped,
            label: String::from("Mempool"),
            cutoff_transaction: None,
        }
    }
}

#[derive(Debug)]
pub struct MempoolModuleComponentConfig {
    pub events_tx: EventPublisher,
    pub mempool_events_rx: EventSubscriber,
}

#[async_trait]
impl RuntimeComponent<MempoolModuleComponentConfig, MempoolReadHandleFactory> for MempoolModule {
    async fn setup(
        args: MempoolModuleComponentConfig,
    ) -> crate::Result<RuntimeComponentHandle<MempoolReadHandleFactory>> {
        let mut mempool_events_rx = args.mempool_events_rx;

        let mempool = LeftRightMempool::new();
        let mempool_read_handle_factory = mempool.factory();

        let mempool_module = MempoolModule::new(MempoolModuleConfig {
            mempool,
            events_tx: args.events_tx,
        });

        let mut mempool_module_actor = ActorImpl::new(mempool_module);

        let mempool_handle = tokio::spawn(async move {
            mempool_module_actor
                .start(&mut mempool_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        let mempool_handle = mempool_handle;

        let component_handle =
            RuntimeComponentHandle::new(mempool_handle, mempool_read_handle_factory);

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}

#[async_trait]
impl Handler<EventMessage> for MempoolModule {
    fn id(&self) -> ActorId {
        self.id.clone()
    }

    fn label(&self) -> ActorLabel {
        self.label.clone()
    }

    fn status(&self) -> ActorState {
        self.status.clone()
    }

    fn on_start(&self) {
        info!("{}-{} starting", self.label(), self.id(),);
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

            Event::NewTxnCreated(txn) => {
                info!("Storing transaction in mempool for validation");

                let txn_hash = txn.id();

                let _mempool_size = self
                    .mempool
                    .insert(txn)
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                self.events_tx
                    .send(Event::TxnAddedToMempool(txn_hash.clone()).into())
                    .await
                    .map_err(|err| TheaterError::Other(err.to_string()))?;

                info!("Transaction {} sent to mempool", txn_hash);

                if self.mempool.size_in_kilobytes() >= MEMPOOL_THRESHOLD_SIZE
                    && self.cutoff_transaction.is_none()
                {
                    info!("mempool threshold reached");
                    self.cutoff_transaction = Some(txn_hash.clone());

                    let event = Event::MempoolSizeThesholdReached {
                        cutoff_transaction: txn_hash,
                    };

                    self.events_tx
                        .send(event.into())
                        .await
                        .map_err(|err| TheaterError::Other(err.to_string()))?;
                }
            },

            Event::TxnValidated(txn) => {
                self.mempool
                    .remove(&txn.id())
                    .map_err(|err| TheaterError::Other(err.to_string()))?;
            },

            Event::NoOp => {},
            _ => {},
        }

        Ok(ActorState::Running)
    }
}
