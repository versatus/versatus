use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

pub use miner::test_helpers::{create_address, create_claim, create_miner};
use primitives::{KademliaPeerId, NodeType};

use uuid::Uuid;
use vrrb_config::{NodeConfig, NodeConfigBuilder, ThresholdConfig};
use vrrb_core::keypair::Keypair;

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
    let prometheus_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let rsa_path = current_dir.join("crates/node/src/test_utils/mocks/sample.rsa");
    let private_key_path = current_dir.join("crates/node/src//test_utils/mocks/sample.pem");

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
        .prometheus_bind_addr(prometheus_addr.ip().to_string())
        .prometheus_bind_port(prometheus_addr.port())
        .prometheus_cert_path(rsa_path.to_str().unwrap().to_string())
        .prometheus_private_key_path(private_key_path.to_str().unwrap().to_string())
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
