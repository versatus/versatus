mod consensus_component;
mod consensus_handler;
mod consensus_module;

mod quorum_module;

pub use consensus_component::*;
pub use consensus_handler::*;
pub use consensus_module::*;
pub use quorum_module::*;

#[cfg(test)]
mod tests {

    use dkg_engine::prelude::{DkgEngine, DkgEngineConfig};
    use events::{AssignedQuorumMembership, Event, DEFAULT_BUFFER};
    use hbbft::crypto::SecretKey as ThresholdSignatureSecretKey;
    use primitives::{KademliaPeerId, NodeType, QuorumKind};
    use theater::Handler;
    use vrrb_config::NodeConfig;
    use vrrb_core::keypair::KeyPair;

    use crate::consensus::{ConsensusModule, ConsensusModuleConfig};
    use crate::test_utils::MockStateReader;

    #[tokio::test]
    #[ignore = "depends on other instances of consensus module to work"]
    async fn consensus_component_can_form_genesis_quorum() {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let node_config = NodeConfig::default();
        let validator_public_key = node_config.keypair.validator_public_key_owned();

        let consensus_module_config = ConsensusModuleConfig {
            events_tx: events_tx.clone(),
            keypair: KeyPair::random(),
            vrrbdb_read_handle: MockStateReader::new(),
            node_config: NodeConfig::default(),
            dkg_generator: DkgEngine::new(DkgEngineConfig {
                node_id: "node 0".into(),
                node_type: NodeType::MasterNode,
                secret_key: ThresholdSignatureSecretKey::random(),
                threshold_config: vrrb_config::ThresholdConfig::default(),
            }),
            validator_public_key,
        };

        let mut consensus_module = ConsensusModule::new(consensus_module_config);

        let event = Event::QuorumMembershipAssigmentCreated(AssignedQuorumMembership {
            node_id: "node_id".to_string(),
            kademlia_peer_id: KademliaPeerId::rand(),
            quorum_kind: QuorumKind::Harvester,
            peers: vec![],
        })
        .into();

        consensus_module.handle(event).await.unwrap();

        events_rx.recv().await.unwrap();
    }
}
