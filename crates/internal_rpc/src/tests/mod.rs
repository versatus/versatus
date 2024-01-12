//! tests must be run serially to avoid failures due to the test socket address being used in every test.

use crate::api::IPFSDataType;
use crate::job_queue::{ComputeJobExecutionType, ServiceJobApi, ServiceJobStatus, ServiceJobType};
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

#[derive(Debug)]
struct TestJob {
    cid: String,
    uuid: uuid::Uuid,
    kind: ServiceJobType,
    inst: std::time::Instant,
    status: ServiceJobStatus,
}
impl ServiceJobApi for TestJob {
    fn new(cid: &str, uuid: uuid::Uuid, kind: crate::job_queue::ServiceJobType) -> Self {
        Self {
            cid: cid.into(),
            uuid,
            kind,
            inst: std::time::Instant::now(),
            status: ServiceJobStatus::Waiting,
        }
    }
    fn cid(&self) -> String {
        self.cid.clone()
    }
    fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
    fn kind(&self) -> crate::job_queue::ServiceJobType {
        self.kind.clone()
    }
    fn inst(&self) -> std::time::Instant {
        self.inst
    }
    fn status(&self) -> ServiceJobStatus {
        self.status.clone()
    }
}

#[tokio::test]
#[serial]
async fn test_start_server() {
    let (handle, _socket) = InternalRpcServer::start::<TestJob>(
        &test_service_config(),
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();

    handle.stop().unwrap();
    let _ = handle;
}

#[tokio::test]
#[serial]
async fn test_client_connection_to_server() {
    let (handle, socket) = InternalRpcServer::start::<TestJob>(
        &test_service_config(),
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    assert!(client.is_connected());

    handle.stop().unwrap();
    let _ = handle;
}

#[tokio::test]
#[serial]
async fn test_get_response_from_server() {
    let service_config = test_service_config();
    let (handle, socket) = InternalRpcServer::start::<TestJob>(
        &service_config,
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    let res = client.0.status().await;
    assert!(res.is_ok());
    dbg!(res.unwrap());

    handle.stop().unwrap();
    let _ = handle;
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_is_object_pinned_from_server() {
    let sample_cid = "bafyreibd2pk7qsmsi5hab6xuvm37qvjlmyjcweiej2dg7nedpd4bwdsgw4";
    let service_config = test_service_config();
    let (handle, socket) = InternalRpcServer::start::<TestJob>(
        &service_config,
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    let res = client.0.is_pinned(sample_cid).await;
    assert_eq!(res.unwrap(), true);
    handle.stop().unwrap();
    let _ = handle;
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_is_retrieve_obj_from_server() {
    let sample_cid = "bafyreibd2pk7qsmsi5hab6xuvm37qvjlmyjcweiej2dg7nedpd4bwdsgw4";
    let service_config = test_service_config();
    let (handle, socket) = InternalRpcServer::start::<TestJob>(
        &service_config,
        platform::services::ServiceType::Storage,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    let res = client
        .0
        .get_data(sample_cid, IPFSDataType::Object)
        .await
        .unwrap();
    assert!(!res.is_empty());
    handle.stop().unwrap();
    let _ = handle;
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_pin_object() {
    let sample_cid = "bafyreibd2pk7qsmsi5hab6xuvm37qvjlmyjcweiej2dg7nedpd4bwdsgw4";
    let service_config = test_service_config();
    let (handle, socket) = InternalRpcServer::start::<TestJob>(
        &service_config,
        platform::services::ServiceType::Storage,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    let res = client.0.pin_object(sample_cid, true).await.unwrap();
    assert!(!res.is_empty());
    handle.stop().unwrap();
    let _ = handle;
}

#[tokio::test]
#[serial]
async fn test_queue_job() {
    let sample_cid = "bafyreibd2pk7qsmsi5hab6xuvm37qvjlmyjcweiej2dg7nedpd4bwdsgw4";
    let service_config = test_service_config();
    let (handle, socket) = InternalRpcServer::start::<TestJob>(
        &service_config,
        platform::services::ServiceType::Compute,
    )
    .await
    .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    let uuid = client
        .0
        .queue_job(
            sample_cid,
            ServiceJobType::Compute(ComputeJobExecutionType::Null),
        )
        .await
        .unwrap();
    let res = client.0.job_status(uuid).await.unwrap();
    assert_eq!(Some(ServiceJobStatus::Waiting), res);

    handle.stop().unwrap();
    let _ = handle;
}
