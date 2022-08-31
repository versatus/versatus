use crate::types::DkgState;
use crate::types::{config::ThresholdConfig, DkgEngine};
use commands::command::Command;
use hbbft::crypto::serde_impl::SerdeSecret;
use hbbft::crypto::{PublicKey, SecretKey};
use messages::packet::Packet;
use node::handler::{CommandHandler, MessageHandler};
use node::node::NodeType;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::RwLock,
};
use tokio::sync::mpsc::unbounded_channel;
use udp2p::protocol::protocol::Message;

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
pub fn generate_key_sets(number_of_nodes: u16) -> (Vec<SecretKey>, BTreeMap<u16, PublicKey>) {
    let sec_keys: Vec<SecretKey> = (0..number_of_nodes).map(|_| rand::random()).collect();
    let pub_keys = sec_keys
        .iter()
        .map(SecretKey::public_key)
        .enumerate()
        .map(|(x, y)| (x as u16, y))
        .collect();
    (sec_keys, pub_keys)
}

/// It generates a DKG engine with a random secret key, a set of public keys, and a command handler
///
/// Arguments:
///
/// * `node_idx`: The index of the node in the network.
/// * `total_nodes`: The total number of nodes in the network.
/// * `node_type`: NodeType - This is the type of node that we want to generate. We can choose from the
/// following:
///
/// Returns:
///
/// A DkgEngine struct with a node_info field that is an Arc<RwLock<Node>>.
pub fn generate_dkg_engines(total_nodes: u16, node_type: NodeType) -> Vec<DkgEngine> {
    let (sec_keys, pub_keys) = generate_key_sets(total_nodes);
    let mut dkg_instances = vec![];
    for i in 0..total_nodes {
        let secret_key: SecretKey = sec_keys.get(i as usize).unwrap().clone();
        let secret_key_encoded = bincode::serialize(&SerdeSecret(secret_key.clone())).unwrap();
        let (_, msg_receiver) = unbounded_channel::<(Packet, std::net::SocketAddr)>();
        let (msg_sender, _) = unbounded_channel();
        dkg_instances.push(DkgEngine {
            node_info: Arc::new(RwLock::new(node::node::Node {
                secret_key: secret_key_encoded,
                pubkey: hex::encode(secret_key.public_key().to_bytes().as_slice()),
                id: i.to_string(),
                node_type: node_type.clone(),
                message_cache: HashSet::default(),
                packet_storage: HashMap::default(),
                command_handler: generate_command_handler(),
                message_handler: MessageHandler::new(msg_sender, msg_receiver),
                idx: i,
            })),
            threshold_config: valid_threshold_config(),
            dkg_state: DkgState {
                part_message_store: HashMap::new(),
                ack_message_store: HashMap::new(),
                peer_public_keys: pub_keys.clone(),
                public_key_set: None,
                secret_key_share: None,
                sync_key_gen: None,
                random_number_gen: None,
                secret_key: secret_key,
            },
        });
    }
    dkg_instances
}

/// It creates a bunch of channels and returns a `CommandHandler` struct that contains all of them
///
/// Returns:
///
/// A CommandHandler struct
fn generate_command_handler() -> CommandHandler {
    let (to_mining_sender, _to_mining_receiver) = unbounded_channel::<Command>();
    let (to_blockchain_sender, _to_blockchain_receiver) = unbounded_channel::<Command>();
    let (to_gossip_sender, _to_gossip_receiver) = unbounded_channel::<Command>();
    let (to_swarm_sender, _to_swarm_receiver) = unbounded_channel::<Command>();
    let (to_state_sender, _to_state_receiver) = unbounded_channel::<Command>();
    let (to_gossip_tx, _to_gossip_rx) = channel::<(std::net::SocketAddr, Message)>();
    let (_sn, rx) = unbounded_channel::<Command>();

    CommandHandler {
        to_mining_sender: to_mining_sender,
        to_blockchain_sender: to_blockchain_sender,
        to_gossip_sender: to_gossip_sender,
        to_swarm_sender: to_swarm_sender,
        to_state_sender: to_state_sender,
        to_gossip_tx: to_gossip_tx,
        receiver: rx,
    }
}
