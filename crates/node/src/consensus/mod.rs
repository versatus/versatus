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
    use std::{
        collections::{HashMap, HashSet},
        net::{IpAddr, Ipv4Addr},
        sync::{Arc, RwLock},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use bulldag::graph::BullDag;
    use dkg_engine::test_utils;
    use events::{AssignedQuorumMembership, Event, EventMessage, JobResult, DEFAULT_BUFFER};
    use hbbft::crypto::SecretKey;
    use primitives::{KademliaPeerId, NodeType, QuorumKind};
    use secp256k1::Message;
    use signer::signer::SignatureProvider;
    use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
    use theater::{Actor, ActorImpl, ActorState, Handler};
    use validator::validator_core_manager::ValidatorCoreManager;
    use vrrb_config::NodeConfig;
    use vrrb_config::ThresholdConfig;
    use vrrb_core::{
        account::Account,
        bloom::Bloom,
        keypair::KeyPair,
        txn::{NewTxnArgs, Txn},
    };

    use crate::test_utils::MockStateReader;
    use crate::{
        consensus::{ConsensusModule, ConsensusModuleConfig},
        test_utils::MockDkgEngine,
    };

    #[tokio::test]
    #[ignore = "depends on other instances of consensus module to work"]
    async fn consensus_component_can_form_genesis_quorum() {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

        let consensus_module_config = ConsensusModuleConfig {
            events_tx: events_tx.clone(),
            keypair: KeyPair::random(),
            vrrbdb_read_handle: MockStateReader::new(),
            node_config: NodeConfig::default(),
            dkg_generator: MockDkgEngine::new(DkgEngineConfig {
                node_idx: 10,
                node_type: NodeType::MasterNode,
                secret_key: SecretKey::random(),
                threshold_config: vrrb_config::ThresholdConfig::default(),
            }),
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
