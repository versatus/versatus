use events::{EventPublisher, EventSubscriber};
use mempool::MempoolReadHandleFactory;
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{Actor, ActorImpl};
use vrrb_config::NodeConfig;

use crate::{node_runtime::NodeRuntime, NodeError, RuntimeComponent, RuntimeComponentHandle};

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
    ) -> crate::Result<RuntimeComponentHandle<NodeRuntimeComponentResolvedData>> {
        let mut events_rx = args.events_rx;
        let node_runtime = NodeRuntime::new(&args.config, args.events_tx).await.map_err(|err| {
            NodeError::Other(err.to_string())
        })?;

        let state_read_handle = node_runtime.state_read_handle();
        let mempool_read_handle_factory = node_runtime.mempool_read_handle_factory();

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
