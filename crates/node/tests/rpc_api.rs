use node::{test_utils::create_mock_full_node_config, Node, NodeType, RuntimeModuleState};
use telemetry::TelemetrySubscriber;
use tokio::sync::mpsc::unbounded_channel;
use vrrb_config::NodeConfig;
use vrrb_core::event_router::Event;
use vrrb_rpc::rpc::{api::RpcClient, client::create_client};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn node_rpc_api_returns_node_type() {
    let mut node_config = create_mock_full_node_config();
    node_config.node_type = NodeType::Bootstrap;

    let (ctrl_tx_1, mut ctrl_rx_1) = unbounded_channel::<Event>();

    let mut vrrb_node = Node::start(&node_config, ctrl_rx_1).await.unwrap();

    let client = create_client(vrrb_node.jsonrpc_server_address())
        .await
        .unwrap();

    assert_eq!(vrrb_node.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        vrrb_node.wait().await.unwrap();
    });

    assert_eq!(client.get_node_type().await.unwrap(), NodeType::Bootstrap);

    ctrl_tx_1.send(Event::Stop).unwrap();

    handle.await.unwrap();
}
