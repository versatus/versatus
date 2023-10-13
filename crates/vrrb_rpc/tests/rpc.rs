use std::{collections::HashMap, net::SocketAddr};

use events::{EventMessage, DEFAULT_BUFFER};
use primitives::{generate_mock_account_keypair, Address};
use secp256k1::Message;
use storage::storage_utils::remove_vrrb_data_dir;
use tokio::sync::mpsc::channel;
use vrrb_core::transactions::{generate_transfer_digest_vec, Token, Transaction, TransactionKind};
use vrrb_rpc::rpc::{
    api::{RpcApiClient, RpcTransactionRecord},
    client::create_client,
    *,
};

mod common;

#[tokio::test]
async fn server_can_publish_transactions_to_be_created() {
    remove_vrrb_data_dir();
    let _: SocketAddr = "127.0.0.1:0"
        .parse()
        .expect("Unable to create Socket Address");

    let (events_tx, _events_rx) = channel::<EventMessage>(DEFAULT_BUFFER);

    // Set up RPC Server to accept connection from client
    let json_rpc_server_config = JsonRpcServerConfig { events_tx, ..Default::default() };

    let (handle, rpc_server_address) = JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    let client = create_client(rpc_server_address).await.unwrap();

    let (secret_key, public_key) = generate_mock_account_keypair();
    let (_, recv_public_key) = generate_mock_account_keypair();

    let address = Address::new(public_key);
    let recv_address = Address::new(recv_public_key);

    let timestamp = 0;
    let sender_address = address.clone();
    let sender_public_key = public_key;
    let amount = 10;
    let nonce = 0;
    let token = Token::default();

    let digest = generate_transfer_digest_vec(
        timestamp,
        sender_address.to_string(),
        sender_public_key,
        recv_address.to_string(),
        token,
        amount,
        nonce,
    );

    type H = secp256k1::hashes::sha256::Hash;
    let msg = Message::from_hashed_data::<H>(&digest);
    let signature = secret_key.sign_ecdsa(msg);

    let txn = TransactionKind::transfer_builder()
        .timestamp(0)
        .sender_address(address.clone())
        .sender_public_key(public_key)
        .receiver_address(recv_address.clone())
        .amount(10)
        .signature(signature)
        .nonce(0)
        .build_kind().expect("failed to build transfer transaction");

    let rec = client.create_txn(txn.clone()).await.unwrap();

    let mock_digest =
        "d9e444c59d773094d8aa755b9f412383ce0c15a99bc342d39b025a7cbf0b3d1a".to_string();

    let mock_record = RpcTransactionRecord {
        id: mock_digest,
        timestamp: 0,
        sender_address: address.clone(),
        sender_public_key: public_key,
        receiver_address: recv_address.clone(),
        token: Token::default(),
        amount: 10,
        signature: signature.to_string().clone(),
        validators: HashMap::new(),
        nonce: 0,
    };

    let result_ser = serde_json::to_string_pretty(&rec).unwrap();
    let mock_ser = serde_json::to_string_pretty(&mock_record).unwrap();

    assert_eq!(result_ser, mock_ser);

    handle.stop().expect("Unable to stop server");
}