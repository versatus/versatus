//! A Versatus compute runtime for running native workloads under an Open Compute Initiative (OCI)
//! runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities};
use anyhow::{Context, Result};
use std::collections::HashMap;

const RUNTIME_DOMAINNAME: &str = "open-compute";
const RUNTIME_PATH: &str = "/usr/bin/runc";

/// A [ComputeRuntime] for being able to execute a native payload using the Open Compute Initiative
/// (OCI) container runtime.
pub struct OpenComputeRuntime {}

impl ComputeRuntime for OpenComputeRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Native
    }

    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    // Specifics to the OCI runtime for getting it to run:
    //  - It requires that the root filesystem directory name is 'rootfs'
    //  - It requires that we pass in --detach if we're going to use the console socket stuff
    //  - It may not need the console socket stuff for some jobs?
    //  - It executes everything through $PATH/sh (or does it?).
    //  - It leaves the container around and needs to be deleted afterwards

    fn setup(&self, job_id: &str, runtime_path: &str) -> Result<()> {
        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "native+x86_64".to_string());

        let mut oci = OciManagerBuilder::default()
            .runtime_path(runtime_path.to_string())
            .oci_runtime(RUNTIME_PATH.to_string())
            .container_payload(vec![
                "/bin/busybox".to_string(),
                "ps".to_string(),
                "ax".to_string(),
            ]) // TODO: This should be the payload we were asked to run
            .container_id(job_id.to_string())
            .domainname(RUNTIME_DOMAINNAME.to_string())
            .hostname(job_id.to_string())
            .annotations(annotations.to_owned())
            .build()
            .context("OCI runtime builder")?;
        oci.prep().context("OCI prep")?;
        oci.spec().context("OCI spec")?;
        oci.execute().context("OCI execute")?;
        Ok(())
    }
}
