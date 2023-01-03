use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use primitives::types::node::NodeType;
use uuid::Uuid;
use vrrb_config::NodeConfig;

pub fn create_mock_full_node_config() -> NodeConfig {
    let data_dir = env::temp_dir();
    let db_path = data_dir.clone().join("node.db");

    let id = Uuid::new_v4().to_simple().to_string();
    let idx = 100;

    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
    let gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let main_bootstrap_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 10)), 8080);
    let bootstrap_node_addresses = vec![
        main_bootstrap_addr,
        main_bootstrap_addr,
        main_bootstrap_addr,
    ];

    NodeConfig {
        id,
        idx,
        data_dir,
        db_path,
        gossip_address,
        node_type: NodeType::Full,
        bootstrap: false,
        bootstrap_node_addresses,
        http_api_address,
        http_api_title: "Node HTTP API".into(),
        http_api_version: "1.0".into(),
        http_api_shutdown_timeout: Some(Duration::from_secs(5)),
    }
}
