#![allow(unused_imports)]
use std::net::SocketAddr;

use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};
use wallet::v2::Wallet;

#[tokio::test]
pub async fn create_wallet_with_rpc_client() {
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    // Set up RPC Server to accept connection from client
    let json_rpc_server_config = JsonRpcServerConfig::default();
    let result = JsonRpcServer::run(&json_rpc_server_config).await;
    if let Ok((handle, _)) = result {
        tokio::spawn(handle.stopped());

        let wallet = Wallet::new(socket_addr.clone()).await;
        println!("{:?}", wallet);
    }
}

#[tokio::test]
pub async fn wallet_sends_txn_to_rpc_server() {
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    // Set up RPC Server to accept connection from client
    let json_rpc_server_config = JsonRpcServerConfig::default();
    let result = JsonRpcServer::run(&json_rpc_server_config).await;

    if let Ok((handle, _)) = result {
        tokio::spawn(handle.stopped());

        let res = Wallet::new(socket_addr.clone()).await;

        if let Ok(mut wallet) = res {
            let tx_res = wallet
                .send_txn(
                    0,
                    "0x192abcdef01234567890fedcba09876543210".to_string(),
                    10,
                    None,
                )
                .await;
            println!("{:?}", tx_res);
        }
    }
}

#[tokio::test]
pub async fn wallet_requests_account_from_rpc_server() {
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    // Set up RPC Server to accept connection from client
    let json_rpc_server_config = JsonRpcServerConfig::default();
    let _ = JsonRpcServer::run(&json_rpc_server_config).await;

    let res = Wallet::new(socket_addr.clone()).await;

    if let Ok(mut wallet) = res {
        let addresses = wallet.addresses.clone();
        if let Some(addr) = addresses.get(&0u32) {
            let rpc_res = wallet.get_account(addr.clone()).await;
            println!("{:?}", rpc_res);
        }
    }
}

#[tokio::test]
pub async fn wallet_sends_create_account_request_to_rpc_server() {
    let socket_addr: SocketAddr = "127.0.0.1:9293"
        .parse()
        .expect("Unable to create Socket Address");

    // Set up RPC Server to accept connection from client
    let json_rpc_server_config = JsonRpcServerConfig::default();
    let _ = JsonRpcServer::run(&json_rpc_server_config).await;

    let res = Wallet::new(socket_addr.clone()).await;

    if let Ok(mut wallet) = res {
        let res = wallet.create_account().await;
        println!("{:?}", res);
    }
}
