use crate::{
    node_runtime::NodeRuntime,
    test_utils::{
        add_group_peer_data_to_node, assign_node_to_quorum, form_groups_with_peer_data,
        handle_assigned_memberships,
    },
};
use events::EventPublisher;
use metric_exporter::metric_factory::PrometheusFactory;
pub use miner::test_helpers::{create_address, create_claim, create_miner};
use primitives::{KademliaPeerId, NodeType, QuorumKind};
use prometheus::labels;
use std::collections::HashMap;
use std::{
    collections::{BTreeMap, VecDeque},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;
use vrrb_config::{BootstrapPeerData, BootstrapQuorumConfig, BootstrapQuorumMember};
use vrrb_core::keypair::Keypair;

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
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let rsa_path = current_dir.join("crates/node/src/test_utils/mocks/sample.rsa");
    let pem_path = current_dir.join("crates/node/src/test_utils/mocks/sample.pem");
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

    let bootstrap_node_config = vrrb_config::BootstrapConfig {
        additional_genesis_receivers: None,
        bootstrap_quorum_config,
    };

    let mut config = create_mock_full_node_config();
    config.id = String::from("node-0");

    config.bootstrap_config = Some(bootstrap_node_config.clone());

    let bootstrap_prometheus_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let factory = Arc::new(
        PrometheusFactory::new(
            bootstrap_prometheus_addr.ip().to_string(),
            bootstrap_prometheus_addr.port(),
            false,
            HashMap::new(),
            rsa_path.to_str().unwrap().to_string(),
            pem_path.to_str().unwrap().to_string(),
            CancellationToken::new(),
        )
        .unwrap(),
    );
    let labels = labels! {
                "service".to_string() => "compute".to_string(),
                "source".to_string() => "versatus".to_string(),
    };
    let node_0 = NodeRuntime::new(&config, events_tx.clone(), factory.clone(), labels.clone())
        .await
        .unwrap();

    let bootstrap_peer_data = BootstrapPeerData {
        id: node_0.config.kademlia_peer_id.unwrap(),
        udp_gossip_addr: node_0.config.udp_gossip_address,
        raptorq_gossip_addr: node_0.config.raptorq_gossip_address,
        kademlia_liveness_addr: node_0.config.kademlia_liveness_address,
    };

    nodes.push_back(node_0);

    for i in 1..=validator_count - 1 {
        let validator_prometheus_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let factory = Arc::new(
            PrometheusFactory::new(
                validator_prometheus_addr.ip().to_string(),
                validator_prometheus_addr.port(),
                false,
                HashMap::new(),
                rsa_path.to_str().unwrap().to_string(),
                pem_path.to_str().unwrap().to_string(),
                CancellationToken::new(),
            )
            .unwrap(),
        );
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

        let node = NodeRuntime::new(&config, events_tx.clone(), factory.clone(), labels.clone())
            .await
            .unwrap();
        nodes.push_back(node);
    }

    for i in validator_count..=validator_count + miner_count {
        let miner_prometheus_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let factory = Arc::new(
            PrometheusFactory::new(
                miner_prometheus_addr.ip().to_string(),
                miner_prometheus_addr.port(),
                false,
                HashMap::new(),
                rsa_path.to_str().unwrap().to_string(),
                pem_path.to_str().unwrap().to_string(),
                CancellationToken::new(),
            )
            .unwrap(),
        );
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

        let miner_node = NodeRuntime::new(
            &miner_config,
            events_tx.clone(),
            factory.clone(),
            labels.clone(),
        )
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
