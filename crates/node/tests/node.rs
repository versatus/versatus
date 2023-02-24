use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use node::{test_utils::create_mock_bootstrap_node_config, Node, NodeType, RuntimeModuleState};
use poem::http::status;
use tokio::sync::mpsc::unbounded_channel;
use uuid::Uuid;
use vrrb_config::NodeConfig;
use vrrb_core::{event_router::Event, keypair::Keypair};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore]
async fn node_runtime_starts_and_stops() {
    let node_config = create_mock_bootstrap_node_config();
    let (bootstrap_ctrl_tx, bootstrap_ctrl_rx) = unbounded_channel::<Event>();
    let bootstrap_node = Node::start(&node_config, bootstrap_ctrl_rx).await.unwrap();

    let data_dir = env::temp_dir();
    let db_path = data_dir.join("node.db");

    let raptorq_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let udp_gossip_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
    let http_api_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082);
    let jsonrpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083);

    let id = Uuid::new_v4().simple().to_string();
    let idx = 100;

    let node_config = NodeConfig {
        id,
        idx,
        data_dir,
        db_path,
        raptorq_gossip_address,
        udp_gossip_address,
        node_type: NodeType::Full,
        bootstrap_node_addresses: bootstrap_node.bootstrap_node_addresses(),
        http_api_address,
        http_api_title: "Node HTTP API".into(),
        http_api_version: "1.0".into(),
        http_api_shutdown_timeout: None,
        jsonrpc_server_address,
        preload_mock_state: false,
        bootstrap_config: None,
        keypair: Keypair::random(),
        disable_networking: false,
    };

    let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
    let mut vrrb_node = Node::start(&node_config, ctrl_rx).await.unwrap();

    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);
    });

    ctrl_tx.send(Event::Stop).unwrap();

    handle.await.unwrap();
}
