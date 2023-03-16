use std::str::FromStr;

use axum::{body::Body, http::Request};
use axum_server::tls_rustls::RustlsConfig;
use hyper::{Client, StatusCode};
use tokio::sync::broadcast::channel;
use vrrb_core::event_router::Event;
use vrrb_rpc::http::*;

mod common;

#[tokio::test]
async fn server_can_list_transactions() {
    let socket_addr: SocketAddr = "127.0.0.1:0"
        .parse()
        .expect("Unable to create Socket Address");

    let (events_tx, events_rx) = unbounded_channel();

    // Set up RPC Server to accept connection from client
    let mut json_rpc_server_config = JsonRpcServerConfig::default();
    json_rpc_server_config.events_tx = events_tx;

    let (handle, socket_addr) = JsonRpcServer::run(&json_rpc_server_config).await.unwrap();
}
