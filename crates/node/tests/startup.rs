use node::{
    test_utils::{
        create_mock_bootstrap_node_config,
        create_mock_full_node_config,
        create_mock_full_node_config_with_bootstrap,
    },
    Node,
    NodeType,
    RuntimeModuleState,
};
use tokio::sync::mpsc::unbounded_channel;
use vrrb_core::event_router::Event;
use vrrb_rpc::rpc::{api::RpcClient, client::create_client};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn node_can_start_as_a_bootstrap_node() {
    let node_config = create_mock_bootstrap_node_config();

    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();

    let vrrb_node = Node::start(&node_config, ctrl_rx_1).await.unwrap();

    let client = create_client(vrrb_node.jsonrpc_server_address())
        .await
        .unwrap();

    assert!(vrrb_node.is_bootsrap());
    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        vrrb_node.wait().await.unwrap();
    });

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    ctrl_tx_1.send(Event::Stop).unwrap();

    handle.await.unwrap();
}

#[tokio::test]
#[serial]
#[ignore]
async fn node_can_join_network() {
    let node_config = create_mock_bootstrap_node_config();

    let (bootstrap_ctrl_tx, bootstrap_ctrl_rx) = unbounded_channel::<Event>();
    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();

    let bootstrap_node = Node::start(&node_config, bootstrap_ctrl_rx).await.unwrap();
    // NOTE: use quick for peer discovery
    let bootstrap_gossip_address = bootstrap_node.udp_gossip_address();

    let node_config_1 = create_mock_full_node_config_with_bootstrap(vec![bootstrap_gossip_address]);
    let node_1 = Node::start(&node_config_1, ctrl_rx_1).await.unwrap();

    let bootstrap_handle = tokio::spawn(async move {
        bootstrap_node.wait().await.unwrap();
    });

    let node_1_handle = tokio::spawn(async move {
        node_1.wait().await.unwrap();
    });

    ctrl_tx_1.send(Event::Stop).unwrap();
    bootstrap_ctrl_tx.send(Event::Stop).unwrap();

    node_1_handle.await.unwrap();
    bootstrap_handle.await.unwrap();
}

#[tokio::test]
#[serial]
async fn bootstrap_node_can_add_newly_joined_peers_to_peer_list() {
    let node_config = create_mock_bootstrap_node_config();

    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();

    let vrrb_node = Node::start(&node_config, ctrl_rx_1).await.unwrap();

    let client = create_client(vrrb_node.jsonrpc_server_address())
        .await
        .unwrap();

    assert!(vrrb_node.is_bootsrap());
    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        vrrb_node.wait().await.unwrap();
    });

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    ctrl_tx_1.send(Event::Stop).unwrap();

    handle.await.unwrap();
}
