use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
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
        node_type: NodeType::Full,
        db_path,
        node_idx: 1,
        bootstrap: false,
        address,
        bootstrap_node_addr: address,
    }
}
