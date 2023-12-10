//! tests must be run serially to avoid failures due to the test socket address being used in every test.

use crate::{api::InternalRpcApiClient, client::InternalRpcClient, server::InternalRpcServer};
use serial_test::serial;
use service_config::ServiceConfig;

fn test_service_config() -> ServiceConfig {
    ServiceConfig {
        name: "test_service".into(),
        rpc_address: "127.0.0.1".into(),
        rpc_port: 8080,
        pre_shared_key: "test".into(),
        tls_private_key_file: "test".into(),
        tls_public_cert_file: "test".into(),
        tls_ca_cert_file: "test".into(),
        exporter_address: "test".into(),
        exporter_port: "test".into(),
    }
}

#[tokio::test]
#[serial]
async fn test_start_server() {
    let (handle, _socket) = InternalRpcServer::start(
        &test_service_config(),
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
#[serial]
async fn test_client_connection_to_server() {
    let (handle, socket) = InternalRpcServer::start(
        &test_service_config(),
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(&socket).await.unwrap();
    assert!(client.is_connected());

    handle.stop().unwrap();
    handle.stopped().await;
    assert!(!client.is_connected());
}

#[tokio::test]
#[serial]
async fn test_get_response_from_server() {
    let (handle, socket) = InternalRpcServer::start(
        &test_service_config(),
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(&socket).await.unwrap();
    assert!(client.0.status().await.is_ok());
    handle.stop().unwrap();
    handle.stopped().await;
}
