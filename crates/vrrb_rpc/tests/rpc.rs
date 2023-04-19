use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use events::{EventMessage, DEFAULT_BUFFER};
use primitives::{generate_account_keypair, generate_mock_account_keypair, Address};
use secp256k1::{rand::rngs::mock, Message};
use tokio::sync::mpsc::{channel, unbounded_channel};
use vrrb_core::txn::{generate_txn_digest_vec, null_txn, NewTxnArgs, Token};
use vrrb_rpc::rpc::{
    api::{RpcApiClient, RpcTransactionRecord},
    client::create_client,
    *,
};

mod common;

#[tokio::test]
async fn server_can_publish_transactions_to_be_created() {
    let socket_addr: SocketAddr = "127.0.0.1:0"
        .parse()
        .expect("Unable to create Socket Address");

    let (events_tx, events_rx) = channel::<EventMessage>(DEFAULT_BUFFER);

    // Set up RPC Server to accept connection from client
    let mut json_rpc_server_config = JsonRpcServerConfig::default();
    json_rpc_server_config.events_tx = events_tx;

    let (handle, rpc_server_address) = JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    let client = create_client(rpc_server_address).await.unwrap();

    let (secret_key, public_key) = generate_mock_account_keypair();

    let address = Address::new(public_key);

    let timestamp = 0;
    let sender_address = address.to_string();
    let sender_public_key = public_key;
    let receiver_address = address.to_string();
    let amount = 10;
    let nonce = 0;
    let token = Token::default();

    let digest = generate_txn_digest_vec(
        timestamp,
        sender_address,
        sender_public_key,
        receiver_address,
        token,
        amount,
        nonce,
    );

    type H = secp256k1::hashes::sha256::Hash;
    let msg = Message::from_hashed_data::<H>(&digest);
    let signature = secret_key.sign_ecdsa(msg);

    let args = NewTxnArgs {
        timestamp: 0,
        sender_address: address.to_string(),
        sender_public_key: public_key,
        receiver_address: address.to_string(),
        token: None,
        amount: 10,
        signature,
        validators: None,
        nonce: 0,
    };

    let rec = client.create_txn(args).await.unwrap();

    let mock_digest =
        "d43e21d53897192f83c2ff701cb538cf5b4d2439b93fae87b30f8ac6f07c20d1".to_string();
    let mock_sender_address =
        "028b0d9b8a79ef99e2d2c030123aef543ffa7e8583e480f229ae7fccd89c8ddbfa".to_string();
    let mock_sender_public_key =
        "028b0d9b8a79ef99e2d2c030123aef543ffa7e8583e480f229ae7fccd89c8ddbfa".to_string();
    let mock_receiver_address =
        "028b0d9b8a79ef99e2d2c030123aef543ffa7e8583e480f229ae7fccd89c8ddbfa".to_string();

    let mock_signature= "30440220029d8f55b771933f5bcea06771cda9fa793478317a5633407366d3f2186ac994022012f5bdc5217f192a34a62e9856c6efd9a0803e7a93bfaefb95783da29c52c3df".to_string();

    let mock_record = RpcTransactionRecord {
        id: mock_digest,
        timestamp: 0,
        sender_address: mock_sender_address,
        sender_public_key: mock_sender_public_key,
        receiver_address: mock_receiver_address,
        token: Token::default(),
        amount: 10,
        signature: mock_signature,
        validators: HashMap::new(),
        nonce: 0,
    };

    let result_ser = serde_json::to_string_pretty(&rec).unwrap();
    let mock_ser = serde_json::to_string_pretty(&mock_record).unwrap();

    assert_eq!(result_ser, mock_ser);

    handle.stop().unwrap();
}
