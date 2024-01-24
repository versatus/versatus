use crate::kontain::KontainRuntime;
use crate::kontain_wasm::KontainWasmRuntime;
use crate::oci_runc::OpenComputeRuntime;
use crate::runtime::{ComputeJobRunner, ComputeRuntime, ComputeRuntimeCapabilities, JobSet};
use crate::youki::YoukiRuntime;
use internal_rpc::job_queue::ComputeJobExecutionType;

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

// This test requires external services to be running, so belongs more in the e2e test
// infrastructure that we're yet to build.
#[test]
fn compute_job_runner() {
    let _ = env_logger::builder().is_test(true).try_init();
    let r = ComputeJobRunner::run(
        &Uuid::new_v4().to_string(),
        "bafyreicrmhglkwxvvesr5bxrpo6slgjthlhhf3l6pfti52ipl733cnvpla", // new contract package
        ComputeJobExecutionType::SmartContract,
        "test",
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
