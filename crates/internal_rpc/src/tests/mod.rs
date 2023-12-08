#![cfg(test)]
use service_config::ServiceConfig;

use crate::{client::InternalRpcClient, server::InternalRpcServer};

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
async fn test_start_server() {
    let (handle, _socket) = InternalRpcServer::start(
        &test_service_config(),
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    assert!(handle.stop().is_ok());
    assert!(handle.is_stopped());
}

#[tokio::test]
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
    assert!(handle.is_stopped());
}
