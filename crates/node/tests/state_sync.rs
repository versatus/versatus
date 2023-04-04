use events::Event;
use jsonrpsee::{core::client::ClientT, ws_client::WsClient};
use node::{test_utils::create_mock_full_node_config, Node, RuntimeModuleState};
use primitives::generate_account_keypair;
use secp256k1::Message;
use telemetry::TelemetrySubscriber;
use tokio::sync::mpsc::unbounded_channel;
use vrrb_core::txn::NewTxnArgs;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::test]
async fn nodes_can_synchronize_state() {
    // TelemetrySubscriber::init(std::io::stdout).unwrap();
    // telemetry::init_tokio_console();

    // NOTE: two instances of a config are required because the node will create a
    // data directory for the database which cannot be the same for both nodes
    let node_config_1 = create_mock_full_node_config();
    let node_config_2 = create_mock_full_node_config();

    let (ctrl_tx_1, ctrl_rx_1) = unbounded_channel::<Event>();
    let (ctrl_tx_2, ctrl_rx_2) = unbounded_channel::<Event>();

    let vrrb_node_1 = Node::start(&node_config_1, ctrl_rx_1).await.unwrap();
    let vrrb_node_2 = Node::start(&node_config_2, ctrl_rx_2).await.unwrap();

    let client_1 = create_client(vrrb_node_1.jsonrpc_server_address())
        .await
        .unwrap();

    let client_2 = create_client(vrrb_node_2.jsonrpc_server_address())
        .await
        .unwrap();

    assert_eq!(vrrb_node_1.status(), RuntimeModuleState::Stopped);
    assert_eq!(vrrb_node_2.status(), RuntimeModuleState::Stopped);

    let handle_1 = tokio::spawn(async move {
        vrrb_node_1.wait().await.unwrap();
    });

    let handle_2 = tokio::spawn(async move {
        vrrb_node_2.wait().await.unwrap();
    });

    for i in 0..100 {
        let (sk, pk) = generate_account_keypair();

        let signature =
            sk.sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb"));

        client_1
            .create_txn(NewTxnArgs {
                timestamp: 0,
                sender_address: String::from("mock sender_address"),
                sender_public_key: pk,
                receiver_address: String::from("mock receiver_address"),
                token: None,
                amount: 0,
                signature,
                nonce: 0,
                validators: None,
            })
            .await
            .unwrap();
    }

    let mempool_snapshot = client_2.get_full_mempool().await.unwrap();

    assert!(!mempool_snapshot.is_empty());

    ctrl_tx_1.send(Event::Stop).unwrap();
    ctrl_tx_2.send(Event::Stop).unwrap();

    handle_1.await.unwrap();
    handle_2.await.unwrap();
}
