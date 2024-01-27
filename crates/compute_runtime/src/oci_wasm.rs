//! A Versatus compute implementation for running a WASM payload (smart contract) under the OCI
//! reference implementation container runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities, JobSet};
use anyhow::{Context, Result};
use log::info;
use std::collections::HashMap;
use std::fs::copy;

const RUNTIME_DOMAINNAME: &str = "oci-wasm";

/// A [ComputeRuntime] designed to execute a Web Assembly (WASM) payload in the Versatus WASM
/// runtime, inside an OCI container runtime.
#[derive(Debug)]
pub struct OciWasmRuntime {}

impl ComputeRuntime for OciWasmRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Wasm
    }

    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    fn execute(&self, job_set: &JobSet, runtime_path: &str, inputs: &str) -> Result<String> {
        // The path to the retrieved contract payload
        let payload_source = format!("{}/{}/contract", &runtime_path, &job_set.payload_id);
        // The path within the container to the contract binary. We copy this below once we have an
        // [OciManager] object to work with.
        let payload_exec_path = "/contract.wasm".to_string();
        // The path to the retrieved OCI container runtime binary
        let runc_path = format!("{}/{}/crun", &runtime_path, &job_set.runtime_id);
        // The path to the versatus-wasm runtime executable outside the container.
        let vwasm_source = format!("{}/{}/versatus-wasm", &runtime_path, &job_set.runtime_id);
        // The path to the versatus-wasm runtime executable within the container (bind mounted).
        let vwasm_dest = format!("/versatus-wasm");

        // base_payload is the start command line to execute within the container. Namely the
        // command line for executing versatus-wasm to execute a container payload.
        let base_payload: Vec<String> = vec![
            vwasm_dest.to_string(),
            "execute".to_string(),
            "--wasm".to_string(),
            payload_exec_path,
            "--meter-limit".to_string(),
            "10000".to_string(), // TODO: hard-coded
            "--json".to_string(),
            "/input.json".to_string(),
        ];
        /*
        let base_payload: Vec<String> = vec![
            km_path,
            "--verbose".to_string(),
            "--km-log-to=/diag/km.log".to_string(),
            "--log-to=/diag/km-guest.log".to_string(),
            "--".to_string(),
            "/bin/busybox".to_string(),
            "ps".to_string(),
            "ax".to_string(),
        ];*/

        let mut linked_files: Vec<(String, String)> = vec![];
        linked_files.push((vwasm_source.to_string(), vwasm_dest.to_string()));
        //linked_files.push(("/bin/busybox".to_string(), "/bin/busybox".to_string()));

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
            .linked_files(Some(linked_files.to_owned()))
            .annotations(annotations.to_owned())
            .build()
            .context("OCI runtime builder")?;
        // This will create the basic filesystem tree for us.
        oci.prep().context("OCI prep")?;

        // Copy the contract into the container
        let payload_dest = format!("{}/contract.wasm", oci.rootfs());
        info!("Copying payload {} to {}", payload_source, payload_dest);
        let _ret = copy(payload_source, payload_dest)?;
        // XXX: hack til we have the real JSON passed in
        //let _ret = copy("/tmp/input.json", format!("{}/input.json", oci.rootfs()))?;

        info!("Generating container spec file");
        oci.spec().context("OCI spec")?;
        info!("Executing job {}", job_set.job_id);
        let output = oci.execute(inputs).context("OCI execute")?;
        Ok(output.to_string())
    }
}

impl OciWasmRuntime {}
