//! A Versatus compute impleentation for running a WASM payload (smart contract) under a Kontain
//! runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities};
use anyhow::{Context, Result};
use std::collections::HashMap;
use telemetry::tracing;

const RUNTIME_DOMAINNAME: &str = "kontain-wasm";
// XXX: This should come from wherever the CID is unpacked
const RUNTIME_PATH: &str = "/home/matthew/tmp/src/kontain-build/pkg/bin/crun";
/// This is the path within the running container under which to find/execute the Kontain Unikernel
/// monitor binary (km).
const KM_EXEC_PATH: &str = "/opt/kontain/bin/km";

/// A [ComputeRuntime] designed to execute a Web Assembly (WASM) payload in the Versatus WASM
/// runtime, inside a Kontain Unikernel container.
#[derive(Debug)]
pub struct KontainWasmRuntime {}

impl ComputeRuntime for KontainWasmRuntime {
    #[telemetry::instrument]
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Wasm
    }

    #[telemetry::instrument]
    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    #[telemetry::instrument]
    fn setup(&self, job_id: &str, runtime_path: &str) -> Result<()> {
        // base_payload is the start command line to execute within the container. Specifically the
        // Kontain Unikernel monitor.
        let base_payload: Vec<String> = vec![
            KM_EXEC_PATH.to_string(),
            "--verbose".to_string(),
            "--km-log-to=/tmp/km.log".to_string(),
            "--output-data=/tmp/km.out".to_string(),
            "--log-to=/tmp/km-guest.log".to_string(),
        ];

        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "unikernel+wasm".to_string());

        let mut oci = OciManagerBuilder::default()
            .runtime_path(runtime_path.to_string())
            .oci_runtime(RUNTIME_PATH.to_string())
            .container_payload(base_payload.to_owned())
            .container_id(job_id.to_string())
            .domainname(RUNTIME_DOMAINNAME.to_string())
            .hostname(job_id.to_string())
            .annotations(annotations.to_owned())
            .build()
            .context("OCI runtime builder")?;
        // This will create the basic filesystem tree for us.
        oci.prep().context("OCI prep")?;
        // We're now responsible for copying in our binaries and any dependencies we need. TODO:
        // Copy the km binary and payload binary into the container root from the package.
        dbg!(oci.rootfs());

        oci.spec().context("OCI spec")?;
        Ok(())
    }
}

impl KontainWasmRuntime {}
