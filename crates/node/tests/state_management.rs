use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use jsonrpsee::{core::client::Subscription, ws_client::WsClientBuilder};
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
use vrrb_rpc::rpc::{
    api::{CreateTxnArgs, RpcClient},
    client::create_client,
};

#[tokio::test]
#[ignore]
async fn can_add_txns_to_mempool() {
    let node_config = create_mock_bootstrap_node_config();

    let (bootstrap_ctrl_tx, bootstrap_ctrl_rx) = unbounded_channel::<Event>();
    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();

    let bootstrap_node = Node::start(&node_config, bootstrap_ctrl_rx).await.unwrap();

    let bootstrap_gossip_address = bootstrap_node.udp_gossip_address();

    let node_config_1 = create_mock_full_node_config_with_bootstrap(vec![bootstrap_gossip_address]);
    let node_1 = Node::start(&node_config_1, ctrl_rx_1).await.unwrap();

    let bootstrap_handle = tokio::spawn(async move {
        bootstrap_node.wait().await.unwrap();
    });

    let client = create_client(node_1.jsonrpc_server_address())
        .await
        .unwrap();

    let node_1_handle = tokio::spawn(async move {
        node_1.wait().await.unwrap();
    });

    client
        .create_txn(CreateTxnArgs {
            sender_address: String::from("mock sender_address"),
            sender_public_key: vec![],
            receiver_address: String::from("mock receiver_address"),
            token: None,
            amount: 0,
            payload: None,
            signature: vec![],
            nonce: 0,
        })
        .await
        .unwrap();

    let mempool_snapshot = client.get_full_mempool().await.unwrap();

    assert!(!mempool_snapshot.is_empty());

    ctrl_tx_1.send(Event::Stop).unwrap();
    bootstrap_ctrl_tx.send(Event::Stop).unwrap();

    node_1_handle.await.unwrap();
    bootstrap_handle.await.unwrap();
}
