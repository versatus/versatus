use std::net::SocketAddr;

use primitives::{PublicKey, SecretKey};
use secp256k1::{generate_keypair, Message, Secp256k1};
use serial_test::serial;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use vrrb_core::{helpers::read_or_generate_keypair_file, keypair::Keypair, txn::Token};
use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};
use wallet::v2::{Wallet, WalletConfig};

#[tokio::test]
#[serial]
pub async fn create_wallet_with_rpc_client() {
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    // Set up RPC Server to accept connection from client
    let json_rpc_server_config = JsonRpcServerConfig::default();

    let (server_handle, _) = JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    tokio::spawn(server_handle.stopped());

    let wallet_config = WalletConfig::default();

    let mut wallet = Wallet::new(wallet_config).await.unwrap();
}

#[tokio::test]
#[serial]
pub async fn wallet_sends_txn_to_rpc_server() {
    // let socket_addr: SocketAddr = "127.0.0.1:9293"
    //     .parse()
    //     .expect("Unable to create Socket Address");

    // let (events_tx, events_rx) = unbounded_channel();

    // // Set up RPC Server to accept connection from client
    // let mut json_rpc_server_config = JsonRpcServerConfig::default();
    // json_rpc_server_config.events_tx = events_tx;

    // let (handle, socket_addr) =
    // JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    // tokio::spawn(handle.stopped());

    // let mut wallet_config = WalletConfig::default();
    // wallet_config.rpc_server_address = socket_addr;

    // let mut wallet = Wallet::new(wallet_config).await.unwrap();

    // type H = secp256k1::hashes::sha256::Hash;

    // let secp = Secp256k1::new();
    // let secret_key = SecretKey::from_hashed_data::<H>(b"vrrb");
    // let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // wallet.create_account(0, public_key).await.unwrap();

    // let timestamp = 0;

    // let txn_digest = wallet
    //     .send_transaction(
    //         0,
    //         "0x192abcdef01234567890fedcba09876543210".to_string(),
    //         10,
    //         Token::default(),
    //         timestamp,
    //     )
    //     .await
    //     .unwrap();

    // assert_eq!(
    //     &txn_digest.to_string(),
    //     "bc2de28cea998b663ed26d1b02e39ecb72c40a5fbba9c2ad66a4f9abd87f360d"
    // );
}

#[tokio::test]
#[serial]
pub async fn wallet_sends_create_account_request_to_rpc_server() {
    // let socket_addr: SocketAddr = "127.0.0.1:9293"
    //     .parse()
    //     .expect("Unable to create Socket Address");

    // let (events_tx, events_rx) = unbounded_channel();

    // // Set up RPC Server to accept connection from client
    // let mut json_rpc_server_config = JsonRpcServerConfig::default();
    // json_rpc_server_config.events_tx = events_tx;

    // let (handle, socket_addr) =
    // JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    // tokio::spawn(handle.stopped());

    // let mut wallet_config = WalletConfig::default();
    // wallet_config.rpc_server_address = socket_addr;

    // let mut wallet = Wallet::new(wallet_config).await.unwrap();

    // let (_, public_key) = generate_keypair(&mut rand::thread_rng());

    // let (address, account) = wallet.create_account(1,
    // public_key).await.unwrap();
}
