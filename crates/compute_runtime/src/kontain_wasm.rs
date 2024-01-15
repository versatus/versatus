//! A Versatus compute implementation for running a WASM payload (smart contract) under a Kontain
//! runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities, JobSet};
use anyhow::{Context, Result};
use log::info;
use std::collections::HashMap;
use std::fs::copy;

const RUNTIME_DOMAINNAME: &str = "kontain-wasm";

/// A [ComputeRuntime] designed to execute a Web Assembly (WASM) payload in the Versatus WASM
/// runtime, inside a Kontain Unikernel container.
#[derive(Debug)]
pub struct KontainWasmRuntime {}

impl ComputeRuntime for KontainWasmRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Wasm
    }

    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    fn execute(&self, job_set: &JobSet, runtime_path: &str) -> Result<String> {
        // The path to the retrieved contract payload
        let payload_source = format!("{}/{}/contract", &runtime_path, &job_set.payload_id);
        // The path within the container to the contract binary. We copy this below once we have an
        // [OciManager] object to work with.
        let _payload_exec_path = "/contract.wasm".to_string();
        // The path to the retrieved Kontain kontainer runtime executable
        let runc_path = format!("{}/{}/krun", &runtime_path, &job_set.runtime_id);
        // The path within the container to the kontain monitor binary.
        let km_path = format!("{}/bin/km", &runtime_path);

        // base_payload is the start command line to execute within the container. Specifically the
        // Kontain Unikernel monitor.
        // TODO: needs to execute the versatus WASM runtime, which in turn needs to execute the
        // smart contract.
        let base_payload: Vec<String> = vec![
            km_path,
            "--verbose".to_string(),
            "--km-log-to=/diag/km.log".to_string(),
            "--output-data=/diag/km.out".to_string(),
            "--log-to=/diag/km-guest.log".to_string(),
        ];

        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "unikernel+wasm".to_string());

        info!("Building OCI object");
        let mut oci = OciManagerBuilder::default()
            .runtime_path(runtime_path.to_string())
            .oci_runtime(runc_path)
            .container_payload(base_payload.to_owned())
            .container_id(job_set.job_id.to_string())
            .domainname(RUNTIME_DOMAINNAME.to_string())
            .hostname(job_set.job_id.to_string())
            .annotations(annotations.to_owned())
            .build()
            .context("OCI runtime builder")?;
        // This will create the basic filesystem tree for us.
        oci.prep().context("OCI prep")?;

        // Copy the contract into the container
        let payload_dest = format!("{}/contract.wasm", oci.rootfs());
        info!("Copying payload {} to {}", payload_source, payload_dest);
        let _ret = copy(payload_source, payload_dest)?;

        info!("Generating container spec file");
        oci.spec().context("OCI spec")?;
        info!("Executing job {}", job_set.job_id);
        oci.execute().context("OCI execute")?;
        Ok("{ \"fake\": \"JSON output\" }".to_string())
    }
}

impl KontainWasmRuntime {}
