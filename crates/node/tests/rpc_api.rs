use events::Event;
use node::{test_utils::create_mock_full_node_config, Node, NodeType, RuntimeModuleState};
use serial_test::serial;
use tokio::sync::mpsc::unbounded_channel;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::test]
#[ignore = "This test is currently ignored ,its broken"]
#[serial]
async fn node_rpc_api_returns_node_type() {
    let mut node_config = create_mock_full_node_config();
    node_config.node_type = NodeType::Bootstrap;

    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();

    let vrrb_node = Node::start(&node_config, ctrl_rx_1).await.unwrap();
    let addr = vrrb_node.jsonrpc_server_address();

    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        vrrb_node.wait().await.unwrap();
    });

    let client = create_client(addr).await.unwrap();

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    ctrl_tx_1.send(Event::Stop).unwrap();

    handle.await.unwrap();
}
