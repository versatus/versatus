use axum::{body::Body, http::Request};
use axum_server::tls_rustls::RustlsConfig;
use hyper::{Client, StatusCode};
use tokio::sync::broadcast::channel;
use vrrb_core::event_router::Event;
use vrrb_rpc::http::*;

mod common;

#[tokio::test]
async fn server_starts_and_stops() {
    let config = HttpApiServerConfig {
        address: "127.0.0.1:0".into(),
        api_title: "Node HTTP API".into(),
        api_version: "1.0".into(),
        server_timeout: None,
        tls_config: None,
    };

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

#[tokio::test]
async fn server_uses_https() {
    let tls_config =
        RustlsConfig::from_pem(common::MOCK_TEST_CERT.into(), common::MOCK_TEST_KEY.into())
            .await
            .unwrap();

    let config = HttpApiServerConfig {
        address: "127.0.0.1:0".into(),
        api_title: "Node HTTP API".into(),
        api_version: "1.0".into(),
        server_timeout: None,
        tls_config: Some(tls_config),
    };

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
                .uri(format!("https://{}/health", addr))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    ctrl_tx.send(Event::Stop).unwrap();
    server_handle.await.unwrap();
}
