use crate::{node_runtime::NodeRuntime, NodeError, RuntimeComponent, RuntimeComponentHandle};
use events::{EventPublisher, EventSubscriber};
use mempool::MempoolReadHandleFactory;
use metric_exporter::metric_factory::PrometheusFactory;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{Actor, ActorImpl};
use tokio::sync::Mutex;
use tokio::time::sleep;
use vrrb_config::NodeConfig;

#[derive(Debug)]
pub struct NodeRuntimeComponentConfig {
    pub config: NodeConfig,
    pub events_tx: EventPublisher,
    pub events_rx: EventSubscriber,
}

#[derive(Debug, Clone)]
pub struct NodeRuntimeComponentResolvedData {
    pub node_config: NodeConfig,
    pub state_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
}

#[async_trait::async_trait]
impl RuntimeComponent<NodeRuntimeComponentConfig, NodeRuntimeComponentResolvedData>
    for NodeRuntime
{
    async fn setup(
        args: NodeRuntimeComponentConfig,
        factory: Arc<PrometheusFactory>,
        labels: HashMap<String, String>,
    ) -> crate::Result<RuntimeComponentHandle<NodeRuntimeComponentResolvedData>> {
        let mut events_rx = args.events_rx;
        let node_runtime = NodeRuntime::new(&args.config, args.events_tx,factory.clone(),labels.clone())
            .await
            .map_err(|err| NodeError::Other(err.to_string()))?;

        let state_read_handle = node_runtime.state_read_handle();
        let mempool_read_handle_factory = node_runtime.mempool_read_handle_factory();
        let unvoted_pending_transactions = factory
            .build_int_gauge(
                "unvoted_pending_transactions",
                "No of pending transactions in mempool",
                labels.clone(),
            )
            .map_err(|e| NodeError::Other(format!("Failed to build prometheus metric :{:?}", e)))?;
        tokio::spawn({
            let cloned_mempool = mempool_read_handle_factory.clone();
            async move {
                loop {
                    unvoted_pending_transactions.set(cloned_mempool.values().len() as i64);
                    sleep(Duration::from_millis(100)).await;
                }
            }
        });
        let mut node_runtime_actor = ActorImpl::new(node_runtime);

        let node_runtime_handle = tokio::spawn(async move {
            node_runtime_actor
                .start(&mut events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        telemetry::info!("NodeRuntime module is operational");

        let node_runtime_resolved_data = NodeRuntimeComponentResolvedData {
            node_config: args.config,
            state_read_handle,
            mempool_read_handle_factory,
        };

        let component_handle = RuntimeComponentHandle::new(
            node_runtime_handle,
            node_runtime_resolved_data,
            String::from("NodeRuntime"),
        );

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}
