use node::test_utils::{create_mock_full_node_config, create_swarm_of_nodes, stop_swarm_of_nodes};
use primitives::{generate_account_keypair, Address, PublicKey, Signature};
use vrrb_core::txn::NewTxnArgs;
use vrrb_rpc::rpc::{api::RpcApiClient, client::create_client};

#[tokio::test]
pub async fn test_node_can_join_and_leave_network() {
    let nodes = create_swarm_of_nodes(9).await;

    let (node_0, _) = nodes.get(0).unwrap();

    let client = create_client(node_0.jsonrpc_server_address())
        .await
        .unwrap();

    dbg!(client.get_node_type().await.unwrap());

    let txs = nodes.iter().map(|(_, tx)| tx.clone()).collect();

    let (sk, pk) = generate_account_keypair();
    let (r_sk, r_pk) = generate_account_keypair();

    let addr = Address::new(pk);
    let r_addr = Address::new(r_pk);

    let args = NewTxnArgs {
        timestamp: chrono::Utc::now().timestamp(),
        sender_address: addr.to_string(),
        sender_public_key: pk,
        receiver_address: r_addr.to_string(),
        token: None,
        amount: 100,
        signature: Signature::from_bytes(&[0; 64]).unwrap(),
        validators: None,
        nonce: TxNonce,
    };

    client.create_txn(args).await.unwrap();

    stop_swarm_of_nodes(txs);

    dbg!("made it here");
}
