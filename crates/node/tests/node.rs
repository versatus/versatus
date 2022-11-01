use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use node::{Node, NodeType, RuntimeModuleState};
use uuid::Uuid;
use vrrb_config::NodeConfig;
use vrrb_core::event_router::Event;

#[tokio::test]
async fn node_runtime_starts_and_stops() {
    let temp_dir_path = env::temp_dir();
    let db_path = temp_dir_path.clone();
    _ = db_path.join("node.db");

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let id = Uuid::new_v4().to_simple().to_string();
    let idx = 100;

    let node_config = NodeConfig {
        id,
        idx,
        data_dir: temp_dir_path,
        node_type: NodeType::Full,
        db_path,
        node_idx: 1,
        bootstrap: false,
        address,
        bootstrap_node_addr: address,
        http_api_address: "127.0.0.1:0".into(),
        http_api_title: "Node HTTP API".into(),
        http_api_version: "1.0".into(),
        http_api_shutdown_timeout: None,
    };

    let mut vrrb_node = Node::new(node_config);

    let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        vrrb_node.start(&mut ctrl_rx).await.unwrap();
        assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);
    });

    ctrl_tx.send(Event::Stop).unwrap();

    handle.await.unwrap();
}
