use std::collections::{BTreeMap, HashMap};

use hbbft::{
    crypto::{serde_impl::SerdeSecret, PublicKey, SecretKey},
    sync_key_gen::Ack,
};
use primitives::{NodeId, NodeType};
use tokio::sync::mpsc::unbounded_channel;

use crate::{
    dkg::DkgGenerator,
    types::{config::ThresholdConfig, DkgEngine, DkgResult},
    DkgState,
    SenderId,
};

pub fn valid_threshold_config() -> ThresholdConfig {
    ThresholdConfig {
        upper_bound: 4,
        threshold: 1,
    }
}

pub fn invalid_threshold_config() -> ThresholdConfig {
    ThresholdConfig {
        upper_bound: 4,
        threshold: 5,
    }
}

/// It generates a vector of secret keys and a map of public keys
///
/// Arguments:
///
/// * `no_of_nodes`: The number of nodes in the network.
pub fn generate_key_sets(number_of_nodes: u16) -> (Vec<SecretKey>, BTreeMap<NodeId, PublicKey>) {
    let sec_keys: Vec<SecretKey> = (0..number_of_nodes).map(|_| rand::random()).collect();
    let pub_keys = sec_keys
        .iter()
        .map(SecretKey::public_key)
        .enumerate()
        .map(|(x, y)| (format!("node-{x}"), y))
        .collect();

    (sec_keys, pub_keys)
}

/// It generates a DKG engine with a random secret key, a set of public keys,
/// and a command handler
///
/// Arguments:
///
/// * `node_idx`: The index of the node in the network.
/// * `total_nodes`: The total number of nodes in the network.
/// * `node_type`: NodeType - This is the type of node that we want to generate.
///   We can choose from the
/// following:
///
/// Returns:
///
/// A DkgEngine struct with a node_info field that is an Arc<RwLock<Node>>.
pub async fn generate_dkg_engines(total_nodes: u16, node_type: NodeType) -> Vec<DkgEngine> {
    let (sec_keys, pub_keys) = generate_key_sets(total_nodes);
    let mut dkg_instances = vec![];
    for i in 0..total_nodes {
        let secret_key: SecretKey = sec_keys.get(i as usize).unwrap().clone();
        let _secret_key_encoded = bincode::serialize(&SerdeSecret(secret_key.clone())).unwrap();

        dkg_instances.push(DkgEngine {
            node_id: format!("node{}", i),
            node_type,
            threshold_config: valid_threshold_config(),
            secret_key: sec_keys.get(i as usize).unwrap().clone(),
            dkg_state: DkgState {
                part_message_store: HashMap::new(),
                ack_message_store: HashMap::new(),
                peer_public_keys: pub_keys.clone(),
                public_key_set: None,
                secret_key_share: None,
                sync_key_generator: None,
                random_number_gen: None,
            },
            harvester_public_key: None,
        });
    }
    dkg_instances
}

pub async fn generate_dkg_engine_with_states() -> Vec<DkgEngine> {
    let mut dkg_engines = generate_dkg_engines(4, NodeType::MasterNode).await;
    let mut dkg_engine_node4 = dkg_engines.pop().unwrap();
    let mut dkg_engine_node3 = dkg_engines.pop().unwrap();
    let mut dkg_engine_node2 = dkg_engines.pop().unwrap();
    let mut dkg_engine_node1 = dkg_engines.pop().unwrap();

    let part_committment_node1 = dkg_engine_node1.generate_sync_keygen_instance(1).unwrap();
    let part_committment_node2 = dkg_engine_node2.generate_sync_keygen_instance(1).unwrap();
    let part_committment_node3 = dkg_engine_node3.generate_sync_keygen_instance(1).unwrap();
    let part_committment_node4 = dkg_engine_node4.generate_sync_keygen_instance(1).unwrap();

    let part_committment_tuples = vec![
        part_committment_node1,
        part_committment_node2,
        part_committment_node3,
        part_committment_node4,
    ];

    for part_commitment in part_committment_tuples.iter() {
        if let DkgResult::PartMessageGenerated(node_id, part) = part_commitment {
            if *node_id != dkg_engine_node1.node_id() {
                dkg_engine_node1
                    .dkg_state
                    .part_message_store
                    .insert(node_id.to_string(), part.clone());
            }
            if *node_id != dkg_engine_node2.node_id() {
                dkg_engine_node2
                    .dkg_state
                    .part_message_store
                    .insert(node_id.to_string(), part.clone());
            }
            if *node_id != dkg_engine_node3.node_id() {
                dkg_engine_node3
                    .dkg_state
                    .part_message_store
                    .insert(node_id.to_string(), part.clone());
            }
            if *node_id != dkg_engine_node4.node_id() {
                dkg_engine_node4
                    .dkg_state
                    .part_message_store
                    .insert(node_id.to_string(), part.clone());
            }
        }
    }

    // let dkg_engine_node1_acks=vec![];
    for i in 0..4 {
        dkg_engine_node1
            .ack_partial_commitment(format!("node-{i}"))
            .unwrap();
        dkg_engine_node2
            .ack_partial_commitment(format!("node-{i}"))
            .unwrap();
        dkg_engine_node3
            .ack_partial_commitment(format!("node-{i}"))
            .unwrap();
        dkg_engine_node4
            .ack_partial_commitment(format!("node-{i}"))
            .unwrap();
    }

    let mut new_store: HashMap<(NodeId, SenderId), Ack>;

    new_store = dkg_engine_node1
        .dkg_state
        .ack_message_store
        .clone()
        .into_iter()
        .chain(dkg_engine_node2.dkg_state.ack_message_store.clone())
        .collect();

    new_store = new_store
        .into_iter()
        .chain(dkg_engine_node3.dkg_state.ack_message_store.clone())
        .collect();

    new_store = new_store
        .into_iter()
        .chain(dkg_engine_node4.dkg_state.ack_message_store.clone())
        .collect();

    dkg_engine_node1.dkg_state.ack_message_store = new_store.clone();
    dkg_engine_node2.dkg_state.ack_message_store = new_store.clone();
    dkg_engine_node3.dkg_state.ack_message_store = new_store.clone();
    dkg_engine_node4.dkg_state.ack_message_store = new_store;

    for _ in 0..4 {
        let _ = dkg_engine_node1.handle_ack_messages();
        let _ = dkg_engine_node2.handle_ack_messages();
        let _ = dkg_engine_node3.handle_ack_messages();
        let _ = dkg_engine_node4.handle_ack_messages();
    }
    let _ = dkg_engine_node1.generate_key_sets();
    let _ = dkg_engine_node2.generate_key_sets();
    let _ = dkg_engine_node3.generate_key_sets();
    let _ = dkg_engine_node4.generate_key_sets();

    // Returning the dkg engines
    vec![
        dkg_engine_node1,
        dkg_engine_node2,
        dkg_engine_node3,
        dkg_engine_node4,
    ]
}
