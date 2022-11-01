use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use primitives::types::NodeType;
use uuid::Uuid;
use vrrb_config::NodeConfig;

pub fn create_mock_full_node_config() -> NodeConfig {
    let temp_dir_path = env::temp_dir();
    let mut db_path = temp_dir_path.clone();
    db_path.join("node.db");

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let id = Uuid::new_v4().to_simple().to_string();
    let idx = 100;

    NodeConfig {
        id,
        idx,
        data_dir: temp_dir_path,
        db_path,
        node_idx: 1,
        address,
        bootstrap: false,
        bootstrap_node_addr: address,
        node_type: NodeType::Full,
        http_api_address: "127.0.0.1:0".into(),
        http_api_title: "Node HTTP API".into(),
        http_api_version: "1.0".into(),
        http_api_shutdown_timeout: Some(Duration::from_secs(5)),
    }
}
