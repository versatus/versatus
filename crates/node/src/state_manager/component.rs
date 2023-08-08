use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use block::Block;
use bulldag::graph::BullDag;
use events::{EventPublisher, EventSubscriber};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use storage::vrrbdb::{VrrbDbConfig, VrrbDbReadHandle};
use telemetry::info;
use theater::{Actor, ActorImpl, Handler};
use vrrb_config::NodeConfig;
use vrrb_core::claim::Claim;

use crate::{
    state_manager::{StateManager, StateManagerConfig},
    NodeError, RuntimeComponent, RuntimeComponentHandle,
};

#[derive(Debug)]
pub struct StateManagerComponentConfig {
    pub events_tx: EventPublisher,
    pub state_events_rx: EventSubscriber,
    pub node_config: NodeConfig,
    pub dag: Arc<RwLock<BullDag<Block, String>>>,
    pub claim: Claim,
}

#[async_trait]
impl RuntimeComponent<StateManagerComponentConfig, (VrrbDbReadHandle, MempoolReadHandleFactory)>
    for StateManager
{
    async fn setup(
        args: StateManagerComponentConfig,
    ) -> crate::Result<RuntimeComponentHandle<(VrrbDbReadHandle, MempoolReadHandleFactory)>> {
        let dag = args.dag;
        let events_tx = args.events_tx;
        let mut state_events_rx = args.state_events_rx;
        let node_config = args.node_config;

        let mut vrrbdb_config = VrrbDbConfig::default();

        if node_config.db_path() != &vrrbdb_config.path {
            vrrbdb_config.with_path(node_config.db_path().to_path_buf());
        }

        let database = storage::vrrbdb::VrrbDb::new(vrrbdb_config);
        let mempool = LeftRightMempool::new();

        let vrrbdb_read_handle = database.read_handle();
        let mempool_read_handle = mempool.factory();

        let state_module = StateManager::new(StateManagerConfig {
            database,
            mempool,
            events_tx,
            dag,
            claim: args.claim,
        });

        let label = state_module.label();

        let mut state_module_actor = ActorImpl::new(state_module);

        let state_handle = tokio::spawn(async move {
            state_module_actor
                .start(&mut state_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        info!("State store is operational");

        let component_handle = RuntimeComponentHandle::new(
            state_handle,
            (vrrbdb_read_handle, mempool_read_handle),
            label,
        );

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}
