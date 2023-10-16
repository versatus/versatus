use node::{
    test_utils::{create_mock_full_node_config, MockStateStore},
    Node, NodeState, RuntimeModuleState,
};
use primitives::{generate_account_keypair, Address};
use secp256k1::Message;
use vrrb_core::transactions::TransactionKind;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::test]
#[ignore = "https://github.com/versatus/versatus/issues/469"]
async fn nodes_can_synchronize_state() {
    // NOTE: two instances of a config are required because the node will create a
    // data directory for the database which cannot be the same for both nodes
    let node_config_1 = create_mock_full_node_config();
    let node_config_2 = create_mock_full_node_config();

    let vrrb_node_1 = Node::start(node_config_1).await.unwrap();
    let vrrb_node_2 = Node::start(node_config_2).await.unwrap();

    let client_1 = create_client(vrrb_node_1.jsonrpc_server_address())
        .await
        .unwrap();

    let client_2 = create_client(vrrb_node_2.jsonrpc_server_address())
        .await
        .unwrap();

    for _ in 0..1_00 {
        let (sk, pk) = generate_account_keypair();
        let (_, recv_pk) = generate_account_keypair();

        let signature =
            sk.sign_ecdsa(Message::from_hashed_data::<secp256k1::hashes::sha256::Hash>(b"vrrb"));

        client_1
            .create_txn(
                TransactionKind::transfer_builder()
                    .timestamp(0)
                    .sender_address(Address::new(pk))
                    .sender_public_key(pk)
                    .receiver_address(Address::new(recv_pk))
                    .amount(0)
                    .signature(signature)
                    .nonce(0)
                    .build_kind()
                    .expect("Unable to build transfer transaction"),
            )
            .await
            .unwrap();
    }

    let mempool_snapshot = client_2.get_full_mempool().await.unwrap();

    assert!(!mempool_snapshot.is_empty());
    assert!(vrrb_node_1.stop().await.unwrap());
    assert!(vrrb_node_2.stop().await.unwrap());
}
