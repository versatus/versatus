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
use primitives::generate_account_keypair;
use secp256k1::Message;
use tokio::sync::mpsc::unbounded_channel;
use vrrb_core::{event_router::Event, txn::NewTxnArgs};
use vrrb_rpc::rpc::{api::RpcClient, client::create_client};

#[tokio::test]
#[ignore]
async fn process_full_node_event_flow() {
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
    let (sk, pk) = generate_account_keypair();

    let signature =
        sk.sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb"));

    client
        .create_txn(NewTxnArgs {
            timestamp: 0,
            sender_address: String::from("mock sender_address"),
            sender_public_key: pk,
            receiver_address: String::from("mock receiver_address"),
            token: None,
            amount: 0,
            payload: None,
            signature,
            nonce: 0,
            validators: None,
        })
        .await
        .unwrap();

    let mempool_snapshot = client.get_full_mempool().await.unwrap();

    assert!(!mempool_snapshot.is_empty());

    ctrl_tx_1.send(Event::Stop).unwrap();
    bootstrap_ctrl_tx.send(Event::Stop).unwrap();

    node_1_handle.await.unwrap();
    bootstrap_handle.await.unwrap();

    panic!();
}
