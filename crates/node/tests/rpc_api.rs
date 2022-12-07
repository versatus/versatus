use node::{test_utils::create_mock_full_node_config, Node, NodeType, RuntimeModuleState};
use tokio::sync::mpsc::unbounded_channel;
use vrrb_config::NodeConfig;
use vrrb_core::event_router::Event;
use vrrb_rpc::rpc::{api::RpcClient, client::create_client};

#[tokio::test]
async fn node_rpc_api_returns_node_type() {
    let node_config = create_mock_full_node_config();

    let (ctrl_tx_1, mut ctrl_rx_1) = unbounded_channel::<Event>();

    let mut vrrb_node = Node::start(&node_config, ctrl_rx_1).await.unwrap();

    let client = create_client(vrrb_node.jsonrpc_server_address())
        .await
        .unwrap();

    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        vrrb_node.wait().await.unwrap();
    });

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Full);

    ctrl_tx_1.send(Event::Stop).unwrap();

    handle.await.unwrap();
}
