use async_trait::async_trait;
use dkg_engine::dkg::DkgGenerator;
use events::{EventPublisher, EventSubscriber};
use hbbft::crypto::PublicKey as ThresholdSignaturePublicKey;
use primitives::ValidatorPublicKey;
use storage::vrrbdb::VrrbDbReadHandle;
use theater::{Actor, ActorImpl};
use vrrb_config::NodeConfig;

use crate::{
    consensus::{ConsensusModule, ConsensusModuleConfig},
    state_reader::StateReader,
    NodeError, RuntimeComponent, RuntimeComponentHandle,
};

#[derive(Debug)]
pub struct ConsensusModuleComponentConfig<K: DkgGenerator + std::fmt::Debug + Send + Sync> {
    pub events_tx: EventPublisher,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub consensus_events_rx: EventSubscriber,
    pub node_config: NodeConfig,
    pub dkg_generator: K,
    pub validator_public_key: ValidatorPublicKey,
}

#[async_trait]
impl<
        S: StateReader + Send + Sync + Clone,
        K: DkgGenerator + std::fmt::Debug + Send + Sync + 'static,
    > RuntimeComponent<ConsensusModuleComponentConfig<K>, ()> for ConsensusModule<S, K>
{
    async fn setup(
        args: ConsensusModuleComponentConfig<K>,
    ) -> crate::Result<RuntimeComponentHandle<()>> {
        let module = ConsensusModule::<VrrbDbReadHandle, K>::new(ConsensusModuleConfig {
            events_tx: args.events_tx,
            vrrbdb_read_handle: args.vrrbdb_read_handle,
            keypair: args.node_config.keypair.clone(),
            node_config: args.node_config.clone(),
            dkg_generator: args.dkg_generator,
            validator_public_key: args.validator_public_key,
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
