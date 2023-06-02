use node::{test_utils::create_mock_full_node_config, Node, NodeType, RuntimeModuleState};
use serial_test::serial;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::test]
#[serial]
async fn node_rpc_api_returns_node_type() {
    let mut node_config = create_mock_full_node_config();
    node_config.node_type = NodeType::Bootstrap;

    let mut vrrb_node = Node::start(&node_config).await.unwrap();
    let addr = vrrb_node.jsonrpc_server_address();

    let client = create_client(addr).await.unwrap();

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    vrrb_node.stop();

    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);
}
