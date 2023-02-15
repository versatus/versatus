use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use primitives::NodeType;
use uuid::Uuid;
use vrrb_config::{NodeConfig, NodeConfigBuilder};
use vrrb_core::keypair::Keypair;

pub fn create_mock_full_node_config() -> NodeConfig {
    let data_dir = env::temp_dir();
    let db_path = data_dir.join("node.db");

    let id = Uuid::new_v4().to_string();
    let idx = 100;

    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let jsonrpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let udp_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let raptorq_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

    let main_bootstrap_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 10)), 0);
    let bootstrap_node_addresses = vec![main_bootstrap_addr];

    NodeConfigBuilder::default()
        .id(id)
        .idx(idx)
        .data_dir(data_dir)
        .db_path(db_path)
        .node_type(NodeType::Full)
        .bootstrap_node_addresses(bootstrap_node_addresses)
        .bootstrap_config(None)
        .http_api_address(http_api_address)
        .http_api_title(String::from("HTTP Node API"))
        .http_api_version(String::from("1.0"))
        .http_api_shutdown_timeout(Some(Duration::from_secs(5)))
        .raptorq_gossip_address(raptorq_gossip_address)
        .udp_gossip_address(udp_gossip_address)
        .jsonrpc_server_address(jsonrpc_server_address)
        .keypair(Keypair::random())
        .build()
        .unwrap()
}

pub fn create_mock_full_node_config_with_bootstrap(
    bootstrap_node_addresses: Vec<SocketAddr>,
) -> NodeConfig {
    let mut node_config = create_mock_full_node_config();

    node_config.bootstrap_node_addresses = bootstrap_node_addresses;
    node_config
}

pub fn create_mock_bootstrap_node_config() -> NodeConfig {
    let mut node_config = create_mock_full_node_config();

    node_config.bootstrap_node_addresses = vec![];
    node_config.node_type = NodeType::Bootstrap;

    node_config
}
