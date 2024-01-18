//! tests must be run serially to avoid failures due to the test socket address being used in every test.

use crate::api::IPFSDataType;
use crate::job_queue::{
    ComputeJobExecutionType, Receiver, ServiceJob, ServiceJobState, ServiceJobType,
    ServiceReceiver, Transmitter,
};
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
    let (handle, _socket, _) =
        InternalRpcServer::start::<Transmitter<ServiceJob>, Receiver<ServiceJob>, ServiceJob>(
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
    let (handle, socket, _) =
        InternalRpcServer::start::<Transmitter<ServiceJob>, Receiver<ServiceJob>, ServiceJob>(
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
    let (handle, socket, _) = InternalRpcServer::start::<
        Transmitter<ServiceJob>,
        Receiver<ServiceJob>,
        ServiceJob,
    >(&service_config, platform::services::ServiceType::Compute)
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
    let (handle, socket, _) = InternalRpcServer::start::<
        Transmitter<ServiceJob>,
        Receiver<ServiceJob>,
        ServiceJob,
    >(&service_config, platform::services::ServiceType::Compute)
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
    let (handle, socket, _) = InternalRpcServer::start::<
        Transmitter<ServiceJob>,
        Receiver<ServiceJob>,
        ServiceJob,
    >(&service_config, platform::services::ServiceType::Storage)
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
    let (handle, socket, _) = InternalRpcServer::start::<
        Transmitter<ServiceJob>,
        Receiver<ServiceJob>,
        ServiceJob,
    >(&service_config, platform::services::ServiceType::Storage)
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
    let (handle, socket, job_queue_rx) =
        InternalRpcServer::start::<Transmitter<ServiceJob>, Receiver<ServiceJob>, ServiceJob>(
            &service_config,
            platform::services::ServiceType::Compute,
        )
        .await
        .unwrap();
    let client = InternalRpcClient::new(socket).await.unwrap();
    let job_handle = std::thread::spawn(move || loop {
        match job_queue_rx.recv() {
            Some(_compute_job) => {}
            None => break,
        }
    });
    let uuid = client
        .0
        .queue_job(
            sample_cid,
            ServiceJobType::Compute(ComputeJobExecutionType::Null),
        )
        .await
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
    let res = client.0.job_status(uuid).await.unwrap().unwrap();
    assert_eq!(ServiceJobState::Received, res.status);
    assert_eq!(3, res.uptime);

    let _ = job_handle;
    handle.stop().unwrap();
    let _ = handle;
}
