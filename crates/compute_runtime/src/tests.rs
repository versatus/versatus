use crate::kontain::KontainRuntime;
use crate::kontain_wasm::KontainWasmRuntime;
use crate::oci_runc::OpenComputeRuntime;
use crate::runtime::{ComputeJobRunner, ComputeRuntime, ComputeRuntimeCapabilities};
use crate::youki::YoukiRuntime;
use internal_rpc::job_queue::ComputeJobExecutionType;

use log::info;
use mktemp::Temp;
use uuid::Uuid;

#[test]
fn check_kontain_wasm_caps() {
    assert_eq!(
        KontainWasmRuntime::capabilities(),
        KontainWasmRuntime::capabilities() & ComputeRuntimeCapabilities::Wasm
    );
}

#[test]
fn check_kontain_caps() {
    assert_eq!(
        KontainRuntime::capabilities(),
        KontainRuntime::capabilities() & ComputeRuntimeCapabilities::Native
    );
}

#[test]
fn check_youki_caps() {
    assert_eq!(
        YoukiRuntime::capabilities(),
        YoukiRuntime::capabilities() & ComputeRuntimeCapabilities::Native
    );
}

#[test]
fn check_oci_runc_caps() {
    assert_eq!(
        OpenComputeRuntime::capabilities(),
        OpenComputeRuntime::capabilities() & ComputeRuntimeCapabilities::Native
    );
}

// The ignored tests below depend on other local binaries that likely won't exist or run on many
// developer workstations. Once we have some of the dependency and build issues solved in this
// repo, we ought to be able to develop tests that have custom payloads that are built inline
// without requiring any external dependencies.

#[test]
fn check_kontain_wasm_setup() {
    let _ = env_logger::builder().is_test(true).try_init();
    info!("Test output");
    let r = KontainWasmRuntime {};
    let path = Temp::new_dir().unwrap();
    let uuid = "0xdeadbeef"; // TODO: This ought to be a UUID and be passed in
    r.setup(&uuid, &path.to_str().unwrap()).unwrap();
    // TODO: Check that temp_dir exists, then drop it and make sure it no longer exists.
    std::mem::drop(r);
}

#[test]
#[ignore]
fn check_oci_runc_exec() {
    let _ = env_logger::builder().is_test(true).try_init();
    let r = OpenComputeRuntime {};
    let path = Temp::new_dir().unwrap();
    let uuid = "0xdeadbeef"; // TODO: This ought to be a UUID and be passed in
    r.setup(&uuid, &path.to_str().unwrap()).unwrap();
    // TODO: Check that temp_dir exists, then drop it and make sure it no longer exists.
}

#[test]
#[ignore]
fn check_youki_exec() {
    let _ = env_logger::builder().is_test(true).try_init();
    let r = YoukiRuntime {};
    let path = Temp::new_dir().unwrap();
    let uuid = "0xdeadbeef"; // TODO: This ought to be a UUID and be passed in
    r.setup(&uuid, &path.to_str().unwrap()).unwrap();
    // TODO: Check that temp_dir exists, then drop it and make sure it no longer exists.
}

#[test]
#[ignore]
fn check_kontain_exec() {
    let _ = env_logger::builder().is_test(true).try_init();
    let r = KontainRuntime {};
    let path = Temp::new_dir().unwrap();
    let uuid = "0xdeadbeef"; // TODO: This ought to be a UUID and be passed in
    r.setup(&uuid, &path.to_str().unwrap()).unwrap();
    // TODO: Check that temp_dir exists, then drop it and make sure it no longer exists.
}

#[test]
fn compute_job_runner_null_true() {
    let _ = env_logger::builder().is_test(true).try_init();
    let r = ComputeJobRunner::run(
        &Uuid::new_v4().to_string(),
        "bafyreide5xf4eou4vkumw2uk7asktbuqe4rqnh4exrgimwdmshi4ok5ene",
        ComputeJobExecutionType::AdHoc,
        &service_config::ServiceConfig {
            name: "storage-test".to_string(),
            rpc_address: "::1".to_string(),
            rpc_port: 9126,
            pre_shared_key: "xxx".to_string(),
            tls_ca_cert_file: "".to_string(),
            tls_private_key_file: "".to_string(),
            tls_public_cert_file: "".to_string(),
            exporter_address: "0.0.0.0".to_string(),
            exporter_port: "9101".to_string(),
        },
    )
    .expect("Job execution failed");
    dbg!(r);
}

#[test]
fn compute_job_runner_null_false() {
    let _ = env_logger::builder().is_test(true).try_init();
    let r = ComputeJobRunner::run(
        &Uuid::new_v4().to_string(),
        crate::runtime::NULL_CID_FALSE,
        ComputeJobExecutionType::AdHoc,
        &service_config::ServiceConfig {
            name: "storage-test".to_string(),
            rpc_address: "::1".to_string(),
            rpc_port: 9126,
            pre_shared_key: "xxx".to_string(),
            tls_ca_cert_file: "".to_string(),
            tls_private_key_file: "".to_string(),
            tls_public_cert_file: "".to_string(),
            exporter_address: "0.0.0.0".to_string(),
            exporter_port: "9101".to_string(),
        },
    )
    .expect("Job execution failed");
    dbg!(r);
}
