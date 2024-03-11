use std::{
    collections::{BTreeMap, HashMap},
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use crate::Node;

use primitives::{KademliaPeerId, NodeId, NodeType, QuorumKind};

use vrrb_config::{
    BootstrapConfig, BootstrapPeerData, BootstrapQuorumConfig, BootstrapQuorumMember, NodeConfig,
    QuorumMember,
};
use vrrb_core::keypair::Keypair;

use super::create_mock_full_node_config;

/// Creates `n` Node instances that make up a network.
pub async fn create_test_network(n: u16) -> Vec<Node> {
    create_test_network_from_config(n, None).await
}

pub async fn create_test_network_from_config(n: u16, base_config: Option<NodeConfig>) -> Vec<Node> {
    let validator_count = (n as f64 * 0.8).ceil() as usize;
    let farmer_count = (validator_count as f64 * 0.7).ceil() as usize;
    let harvester_count = validator_count - farmer_count;
    let miner_count = n as usize - validator_count;

    let mut nodes = vec![];
    let mut bootstrap_quorum_members = BTreeMap::new();
    let mut keypairs = vec![];

    let mut port = 9090;
    for i in 1..=n {
        let udp_port: u16 = 11000 + i;
        let raptor_port: u16 = 12000 + i;
        let kademlia_port: u16 = 13000 + i;

        let keypair = Keypair::random();
        let validator_public_key = keypair.miner_public_key_owned();

        keypairs.push(keypair);

        let node_id = format!("node-{}", i);

        let quorum_kind = if usize::from(i) < farmer_count {
            QuorumKind::Farmer
        } else if usize::from(i) < farmer_count + harvester_count {
            QuorumKind::Harvester
        } else {
            QuorumKind::Miner
        };

        let node_type = if usize::from(i) < farmer_count + harvester_count {
            NodeType::Validator
        } else {
            NodeType::Miner
        };

        let member = BootstrapQuorumMember {
            node_id: node_id.clone(),
            kademlia_peer_id: KademliaPeerId::rand(),
            quorum_kind,
            node_type,
            udp_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), udp_port),
            raptorq_gossip_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), raptor_port),
            kademlia_liveness_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                kademlia_port,
            ),
            validator_public_key,
        };

        bootstrap_quorum_members.insert(node_id, member);
    }

    let quorum_members = bootstrap_quorum_members
        .iter()
        // .cloned()
        .map(|(k, v)| (k.clone(), v.clone().into()))
        .collect::<HashMap<NodeId, QuorumMember>>();

    let whitelisted_nodes = bootstrap_quorum_members
        .values()
        .cloned()
        .map(|v| v.into())
        .collect::<Vec<QuorumMember>>();

    let bootstrap_quorum_config = BootstrapQuorumConfig {
        quorum_members: bootstrap_quorum_members.clone(),
    };

    let additional_genesis_receivers = if let Some(base_config) = base_config.clone() {
        if let Some(base_config) = base_config.bootstrap_config {
            base_config.additional_genesis_receivers
        } else {
            None
        }
    } else {
        None
    };

    let bootstrap_config = BootstrapConfig {
        additional_genesis_receivers,
        bootstrap_quorum_config: bootstrap_quorum_config.clone(),
    };

    let mut config = create_mock_full_node_config();
    config.id = String::from("node-0");
    config.bootstrap_config = Some(bootstrap_config.clone());
    config.whitelisted_nodes = whitelisted_nodes.clone();
    config.prometheus_bind_port = port;

    let node_0 = Node::start(config).await.unwrap();

    let bootstrap_peer_data = BootstrapPeerData {
        id: node_0.kademlia_peer_id(),
        udp_gossip_addr: node_0.udp_gossip_address(),
        raptorq_gossip_addr: node_0.raptorq_gossip_address(),
        kademlia_liveness_addr: node_0.kademlia_liveness_address(),
    };

    nodes.push(node_0);

    for i in 1..=validator_count - 1 {
        port += i as u16;
        let mut config = create_mock_full_node_config();
        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();
        config.id = format!("node-{}", i);
        config.keypair = keypairs.get(i - 1).unwrap().clone();
        config.bootstrap_peer_data = Some(bootstrap_peer_data.clone());
        config.node_type = NodeType::Validator;
        config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        config.udp_gossip_address = quorum_config.udp_gossip_address;
        config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);
        config.whitelisted_nodes = whitelisted_nodes.clone();
        config.prometheus_bind_addr = String::from("127.0.0.1");
        config.prometheus_bind_port = port;
        if let Some(base_config) = &base_config {
            config.enable_ui = base_config.enable_ui;
        }

        let node = Node::start(config).await.unwrap();
        nodes.push(node);
    }

    for i in validator_count..=validator_count + miner_count {
        port += i as u16;
        let mut miner_config = create_mock_full_node_config();
        let node_id = format!("node-{}", i);
        let quorum_config = quorum_members.get(&node_id).unwrap().to_owned();
        miner_config.id = format!("node-{}", i);
        miner_config.keypair = keypairs.get(i - 1).unwrap().clone();
        miner_config.bootstrap_peer_data = Some(bootstrap_peer_data.clone());
        miner_config.node_type = NodeType::Miner;
        miner_config.kademlia_liveness_address = quorum_config.kademlia_liveness_address;
        miner_config.raptorq_gossip_address = quorum_config.raptorq_gossip_address;
        miner_config.udp_gossip_address = quorum_config.udp_gossip_address;
        miner_config.kademlia_peer_id = Some(quorum_config.kademlia_peer_id);
        miner_config.whitelisted_nodes = whitelisted_nodes.clone();
        miner_config.prometheus_bind_port = port;
        if let Some(base_config) = &base_config {
            miner_config.enable_ui = base_config.enable_ui;
        }

        let miner_node = Node::start(miner_config).await.unwrap();

        nodes.push(miner_node);
    }

    nodes
}
