//! A Versatus compute runtime for running native workloads under an Open Compute Initiative (OCI)
//! runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities, JobSet};
use anyhow::{Context, Result};
use std::collections::HashMap;

const RUNTIME_DOMAINNAME: &str = "open-compute";

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

    fn execute(&self, job_set: &JobSet, runtime_path: &str) -> Result<String> {
        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "native+x86_64".to_string());

        let payload_path = format!("{}/{}/payload", &runtime_path, &job_set.payload_id);
        let runc_path = format!("{}/{}/crun", &runtime_path, &job_set.runtime_id);

        // We don't allow any command line arguments to be passed in to the payload. This could
        // open us up to a bunch of different shell escaping exploits. It is likely that any user
        // input for native-executed payloads will also be pushed into stdin in the future, and
        // we'll reserve the command line arguments and environment variables for the runtime being
        // able to provide sanitised information to the runnable payload.

        let mut oci = OciManagerBuilder::default()
            .runtime_path(runtime_path.to_string())
            .oci_runtime(runc_path)
            .container_payload(vec![payload_path])
            .container_id(job_set.job_id.to_string())
            .domainname(RUNTIME_DOMAINNAME.to_string())
            .hostname(job_set.job_id.to_string())
            .annotations(annotations.to_owned())
            .build()
            .context("OCI runtime builder")?;
        oci.prep().context("OCI prep")?;
        oci.spec().context("OCI spec")?;
        let output = oci.execute().context("OCI spec")?;
        Ok(output.to_string())
    }
}
