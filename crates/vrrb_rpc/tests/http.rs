use poem::{
    get,
    listener::{Acceptor, TcpAcceptor},
    Endpoint,
};
use tokio::{signal::unix::SignalKind, sync::mpsc::channel};

use poem::test::TestClient;
use poem::Route;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use vrrb_rpc::http::*;

#[tokio::test]
async fn server_starts_and_stops() {
    let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let acceptor = TcpAcceptor::from_std(listener).unwrap();
    let addr = acceptor.local_addr().get(0).unwrap();

    let config = HttpApiServerConfig {
        acceptor,
        api_title: "Node HTTP API".into(),
        api_version: "1.0".into(),
        server_timeout: None,
    };

    let api = HttpApiServer::new(config).unwrap();

    let (ctrl_tx, mut ctrl_rx) = channel(1);

    let server_handle = tokio::spawn(async move {
        api.start(&mut ctrl_rx).await.unwrap();
    });

    ctrl_tx.send(()).await.unwrap();
    server_handle.await.unwrap();
}
