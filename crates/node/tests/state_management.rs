use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use jsonrpsee::{core::client::Subscription, ws_client::WsClientBuilder};
use node::{test_utils::create_mock_full_node_config, Node, NodeType, RuntimeModuleState};
use tokio::sync::mpsc::unbounded_channel;
use vrrb_config::NodeConfig;
use vrrb_core::event_router::Event;
use vrrb_rpc::rpc::{
    api::{CreateTxnArgs, RpcClient},
    client::create_client,
};

#[tokio::test]
async fn can_add_txns_to_mempool() {
    let node_config = create_mock_full_node_config();

    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();

    let vrrb_node_1 = Node::start(&node_config, ctrl_rx_1).await.unwrap();

    let client = create_client(vrrb_node_1.jsonrpc_server_address())
        .await
        .unwrap();

    assert_eq!(vrrb_node_1.status(), RuntimeModuleState::Stopped);

    let handle_1 = tokio::spawn(async move {
        vrrb_node_1.wait().await.unwrap();
    });

    ctrl_tx_1.send(Event::Stop).unwrap();

    client
        .create_txn(CreateTxnArgs {
            sender_address: String::from("mock sender_address"),
            sender_public_key: vec![],
            receiver_address: Some::from("mock receiver_address"),
            token: None,
            amount: 0,
            payload: None,
            signature: vec![],
            nonce: 0,
        })
        .await
        .unwrap();

    handle_1.await.unwrap();
}
