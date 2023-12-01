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

use crate::{
    network::NetworkEvent,
    node_runtime::NodeRuntime,
    test_utils::{
        add_group_peer_data_to_node, assign_node_to_quorum, form_groups_with_peer_data,
        handle_assigned_memberships,
    },
    Node, Result,
};
use events::{AssignedQuorumMembership, EventPublisher, PeerData, DEFAULT_BUFFER};
pub use miner::test_helpers::{create_address, create_claim, create_miner};
use primitives::{generate_account_keypair, Address, KademliaPeerId, NodeId, NodeType, QuorumKind};
use rand::{seq::SliceRandom, thread_rng};
use secp256k1::{Message, PublicKey, SecretKey};
use sha256::digest;
use signer::engine::SignerEngine;
use uuid::Uuid;
use vrrb_config::{
    BootstrapPeerData, BootstrapQuorumConfig, BootstrapQuorumMember, NodeConfig, NodeConfigBuilder,
    QuorumMember, QuorumMembershipConfig, ThresholdConfig,
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

use super::create_mock_full_node_config;
/// Creates n NodeRuntimes to simulate networks
pub async fn create_node_runtime_network(
    n: usize,
    events_tx: EventPublisher,
) -> VecDeque<NodeRuntime> {
    let validator_count = (n as f64 * 0.8).ceil() as usize;
    let miner_count = n - validator_count;

    let mut nodes = VecDeque::new();

    let mut quorum_members = BTreeMap::new();

    for i in 1..=n as u16 {
        let udp_port: u16 = 11000 + i;
        let raptor_port: u16 = 12000 + i;
        let kademlia_port: u16 = 13000 + i;
        let keypair = Keypair::random();
        let validator_public_key = keypair.miner_public_key_owned();

        let node_id = format!("node-{}", i);

        let member = BootstrapQuorumMember {
            node_id: node_id.clone(),
            kademlia_peer_id: KademliaPeerId::rand(),
            node_type: NodeType::Validator,
            quorum_kind: QuorumKind::Farmer,
            udp_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), udp_port),
            raptorq_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), raptor_port),
            kademlia_liveness_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                kademlia_port,
            ),
            validator_public_key,
        };

        quorum_members.insert(node_id, member);
    }

    let bootstrap_quorum_config = BootstrapQuorumConfig {
        quorum_members: quorum_members.clone(),
    };

    let mut bootstrap_node_config = vrrb_config::BootstrapConfig {
        additional_genesis_receivers: None,
        bootstrap_quorum_config,
    };

    let mut config = create_mock_full_node_config();
    config.id = String::from("node-0");

    config.bootstrap_config = Some(bootstrap_node_config.clone());

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

    let node_0 = NodeRuntime::new(&config, events_tx.clone()).await.unwrap();

    let mut bootstrap_peer_data = BootstrapPeerData {
        id: node_0.config.kademlia_peer_id.clone().unwrap(),
        udp_gossip_addr: node_0.config.udp_gossip_address,
        raptorq_gossip_addr: node_0.config.raptorq_gossip_address,
        kademlia_liveness_addr: node_0.config.kademlia_liveness_address,
    };

    nodes.push_back(node_0);

    for i in 1..=validator_count - 1 {
        let mut config = create_mock_full_node_config();

        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();

        config.id = format!("node-{}", i);
        config.bootstrap_config = Some(bootstrap_node_config.clone());
        config.bootstrap_peer_data = Some(bootstrap_peer_data.clone());
        config.node_type = NodeType::Validator;
        config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        config.udp_gossip_address = quorum_config.udp_gossip_address;
        config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);

        let node = NodeRuntime::new(&config, events_tx.clone()).await.unwrap();
        nodes.push_back(node);
    }

    for i in validator_count..=validator_count + miner_count {
        let mut miner_config = create_mock_full_node_config();

        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();

        miner_config.id = format!("node-{}", i);
        config.bootstrap_config = Some(bootstrap_node_config.clone());
        config.bootstrap_peer_data = Some(bootstrap_peer_data.clone());
        miner_config.node_type = NodeType::Miner;
        miner_config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        miner_config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        miner_config.udp_gossip_address = quorum_config.udp_gossip_address;
        miner_config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);

        let miner_node = NodeRuntime::new(&miner_config, events_tx.clone())
            .await
            .unwrap();

        nodes.push_back(miner_node);
    }

    nodes
}

pub async fn create_quorum_assigned_node_runtime_network(
    n: usize,
    min_quorum_size: usize,
    events_tx: EventPublisher,
) -> Vec<NodeRuntime> {
    assert!(n > (1 + (min_quorum_size * 2)));
    let mut nodes = create_node_runtime_network(n, events_tx.clone()).await;
    // NOTE: remove bootstrap
    nodes.pop_front().unwrap();

    let mut quorums = vec![];
    form_groups_with_peer_data(&mut nodes, min_quorum_size, &mut quorums);
    add_group_peer_data_to_node(&mut quorums).await;
    let mut assigned_memberships = vec![];
    assign_node_to_quorum(&quorums, &mut assigned_memberships);
    let mut quorums_only = quorums.into_iter().map(|(nr, _)| nr).collect();
    handle_assigned_memberships(&mut quorums_only, assigned_memberships);
    quorums_only.into_iter().flatten().collect()
}
