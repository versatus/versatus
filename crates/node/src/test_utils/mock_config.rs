use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap, HashSet, VecDeque},
    env,
    hash::{Hash, Hasher},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, RwLock},
    time::Duration,
};

use block::{
    header::BlockHeader, Block, BlockHash, ConvergenceBlock, GenesisBlock, InnerBlock,
    ProposalBlock,
};
use bulldag::{graph::BullDag, vertex::Vertex};
use quorum::{election::Election, quorum::Quorum};

use crate::{network::NetworkEvent, node_runtime::NodeRuntime, Node, Result};
use events::{AssignedQuorumMembership, EventPublisher, PeerData, DEFAULT_BUFFER};
pub use miner::test_helpers::{create_address, create_claim, create_miner};
use primitives::{generate_account_keypair, Address, KademliaPeerId, NodeId, NodeType, QuorumKind};
use rand::{seq::SliceRandom, thread_rng};
use secp256k1::{Message, PublicKey, SecretKey};
use sha256::digest;
use signer::engine::SignerEngine;
use uuid::Uuid;
use vrrb_config::{
    BootstrapQuorumConfig, NodeConfig, NodeConfigBuilder, QuorumMember, QuorumMembershipConfig,
    ThresholdConfig,
};
use vrrb_core::{
    account::{Account, AccountField},
    claim::Claim,
    keypair::{KeyPair, Keypair},
    transactions::{
        generate_transfer_digest_vec, NewTransferArgs, Transaction, TransactionDigest,
        TransactionKind, Transfer,
    },
};
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

pub fn create_mock_full_node_config() -> NodeConfig {
    let data_dir = env::temp_dir();
    let id = Uuid::new_v4().simple().to_string();

    let temp_dir_path = std::env::temp_dir();
    let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let jsonrpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let rendezvous_local_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let rendezvous_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let public_ip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let udp_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let raptorq_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let kademlia_liveness_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

    NodeConfigBuilder::default()
        .id(id)
        .data_dir(data_dir)
        .db_path(db_path)
        .node_type(NodeType::Bootstrap)
        .bootstrap_config(None)
        .bootstrap_peer_data(None)
        .http_api_address(http_api_address)
        .http_api_title(String::from("HTTP Node API"))
        .http_api_version(String::from("1.0"))
        .http_api_shutdown_timeout(Some(Duration::from_secs(5)))
        .jsonrpc_server_address(jsonrpc_server_address)
        .keypair(Keypair::random())
        .rendezvous_local_address(rendezvous_local_address)
        .rendezvous_server_address(rendezvous_server_address)
        .udp_gossip_address(udp_gossip_address)
        .raptorq_gossip_address(raptorq_gossip_address)
        .kademlia_peer_id(Some(KademliaPeerId::rand()))
        .kademlia_liveness_address(kademlia_liveness_address)
        .public_ip_address(public_ip_address)
        .disable_networking(false)
        .quorum_config(None)
        .threshold_config(ThresholdConfig::default())
        .whitelisted_nodes(vec![])
        .build()
        .unwrap()
}

#[deprecated]
pub fn create_mock_full_node_config_with_bootstrap(
    _bootstrap_node_addresses: Vec<SocketAddr>,
) -> NodeConfig {
    create_mock_full_node_config()
}

#[deprecated]
pub fn create_mock_bootstrap_node_config() -> NodeConfig {
    create_mock_full_node_config()
}
