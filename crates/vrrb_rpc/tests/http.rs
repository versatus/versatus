use crate::{HttpApiServer, HttpApiServerConfig, HttpApiServerConfigBuilder};
use axum::{body::Body, http::Request};

use events::Event;
use hyper::{Client, StatusCode};
use tokio::sync::broadcast::channel;
use vrrb_rpc::http::*;

mod common;

#[tokio::test]
async fn server_starts_and_stops() {
    let config: HttpApiServerConfig = HttpApiServerConfigBuilder::default()
        .address("127.0.0.1:0")
        .api_title("Node HTTP API")
        .api_version("1.0")
        .server_timeout(None)
        .tls_config(None)
        .build();

    let api = HttpApiServer::new(config).unwrap();

    let (ctrl_tx, mut ctrl_rx) = channel(1);

    let addr = api.address().unwrap();

    let server_handle = tokio::spawn(async move {
        api.start(&mut ctrl_rx).await.unwrap();
    });

    let client = Client::new();

    let response = client
        .request(
            Request::builder()
                .uri(format!("http://{}/health", addr))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    ctrl_tx.send(Event::Stop).unwrap();
    server_handle.await.unwrap();
}
