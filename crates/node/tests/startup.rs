use node::test_utils;
use node::{
    test_utils::{
        create_mock_bootstrap_node_config, create_mock_full_node_config_with_bootstrap,
        MockStateStore,
    },
    Node,
};
use primitives::node::NodeType;
use serial_test::serial;
use storage::storage_utils::remove_vrrb_data_dir;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::test]
#[serial]
async fn node_can_start_as_a_bootstrap_node() {
    remove_vrrb_data_dir();
    let node_config = create_mock_bootstrap_node_config();

    let mut vrrb_node = Node::start(node_config).await.unwrap();

    let client = create_client(vrrb_node.jsonrpc_server_address())
        .await
        .unwrap();

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    assert!(vrrb_node.is_bootstrap());
    let is_cancelled = vrrb_node.stop().await.unwrap();

    assert!(is_cancelled);
}

#[tokio::test]
#[serial]
async fn bootstrap_node_can_add_newly_joined_peers_to_peer_list() {
    remove_vrrb_data_dir();
    let node_config = create_mock_bootstrap_node_config();

    let mut vrrb_node = Node::start(node_config).await.unwrap();

    let client = create_client(vrrb_node.jsonrpc_server_address())
        .await
        .unwrap();

    assert!(vrrb_node.is_bootstrap());
    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    let is_cancelled = vrrb_node.stop().await.unwrap();
    assert!(is_cancelled);
}
