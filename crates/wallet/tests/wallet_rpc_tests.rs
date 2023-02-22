use std::net::SocketAddr;

use primitives::{PublicKey, SecretKey};
use secp256k1::Secp256k1;
use serial_test::serial;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use vrrb_core::keypair::Keypair;
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
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    let (events_tx, events_rx) = unbounded_channel();

    // Set up RPC Server to accept connection from client
    let mut json_rpc_server_config = JsonRpcServerConfig::default();
    json_rpc_server_config.events_tx = events_tx;

    let (handle, socket_addr) = JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    tokio::spawn(handle.stopped());

    let mut wallet_config = WalletConfig::default();
    wallet_config.rpc_server_address = socket_addr;

    let mut wallet = Wallet::new(wallet_config).await.unwrap();

    let timestamp = 0;

    let txn_digest = wallet
        .send_transaction(
            0,
            "0x192abcdef01234567890fedcba09876543210".to_string(),
            10,
            None,
            timestamp,
        )
        .await
        .unwrap();

    assert_eq!(
        &txn_digest.to_string(),
        "54e2c0b70ec83d5fd9fa1a11c4bc91d0547fe77bebbc14902e7e3e32bfe42086"
    );
}

#[tokio::test]
#[serial]
pub async fn wallet_sends_create_account_request_to_rpc_server() {
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    let (events_tx, events_rx) = unbounded_channel();

    // Set up RPC Server to accept connection from client
    let mut json_rpc_server_config = JsonRpcServerConfig::default();
    json_rpc_server_config.events_tx = events_tx;

    let (handle, socket_addr) = JsonRpcServer::run(&json_rpc_server_config).await.unwrap();

    tokio::spawn(handle.stopped());

    let mut wallet_config = WalletConfig::default();
    wallet_config.rpc_server_address = socket_addr;

    let mut wallet = Wallet::new(wallet_config).await.unwrap();

    let (address, account) = wallet.create_account().await.unwrap();

    wallet.get_account(address.clone()).await.unwrap();
}
