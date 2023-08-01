use async_trait::async_trait;
use events::{EventPublisher, EventSubscriber};
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{Actor, ActorImpl};
use vrrb_config::NodeConfig;

use crate::{
    consensus::{ConsensusModule, ConsensusModuleConfig},
    NodeError,
    RuntimeComponent,
    RuntimeComponentHandle,
};

#[derive(Debug)]
pub struct ConsensusModuleComponentConfig {
    pub events_tx: EventPublisher,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub consensus_events_rx: EventSubscriber,
    pub node_config: NodeConfig,
}

#[async_trait]
impl RuntimeComponent<ConsensusModuleComponentConfig, ()> for ConsensusModule {
    async fn setup(
        args: ConsensusModuleComponentConfig,
    ) -> crate::Result<RuntimeComponentHandle<()>> {
        let module = ConsensusModule::new(ConsensusModuleConfig {
            events_tx: args.events_tx,
            vrrbdb_read_handle: args.vrrbdb_read_handle,
            keypair: args.node_config.keypair,
        });

        let mut consensus_events_rx = args.consensus_events_rx;
        let mut consensus_module_actor = ActorImpl::new(module);
        let label = consensus_module_actor.label();
        let consensus_handle = tokio::spawn(async move {
            consensus_module_actor
                .start(&mut consensus_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        let component_handle = RuntimeComponentHandle::new(consensus_handle, (), label);

        Ok(component_handle)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        todo!()
    }
}
