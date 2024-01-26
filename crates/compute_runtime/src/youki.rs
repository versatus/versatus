//! A Versatus compute runtime for running a native workload under the Youki container runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities, JobSet};
use anyhow::{Context, Result};
use std::collections::HashMap;

const RUNTIME_DOMAINNAME: &str = "youki";

/// A [ComputeRuntime] for being able to execute a native workload within the Youki container
/// runtime.
pub struct YoukiRuntime {}

impl ComputeRuntime for YoukiRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Native
    }

    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    fn execute(&self, job_set: &JobSet, runtime_path: &str) -> Result<String> {
        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "native+x86_64".to_string());

        // Give the path to the binary within the provided web3 package to the container runtime
        // binary.
        let runtime_bin = format!("{}/{}/runc", runtime_path, job_set.runtime_id);
        // Give the path to the payload binary within the provided web3 package.
        let payload_bin = format!("{}/{}/payload", runtime_path, job_set.payload_id);

        // We don't allow any command line arguments to be passed in to the payload. This could
        // open us up to a bunch of different shell escaping exploits. It is likely that any user
        // input for native-executed payloads will also be pushed into stdin in the future, and
        // we'll reserve the command line arguments and environment variables for the runtime being
        // able to provide sanitised information to the runnable payload.

        let mut oci = OciManagerBuilder::default()
            .runtime_path(runtime_path.to_string())
            .oci_runtime(runtime_bin)
            .container_payload(vec![payload_bin])
            .container_id(job_set.job_id.to_string())
            .domainname(RUNTIME_DOMAINNAME.to_string())
            .hostname(job_set.job_id.to_string())
            .annotations(annotations.to_owned())
            .build()
            .context("OCI runtime builder")?;
        oci.prep().context("OCI prep")?;
        oci.spec().context("OCI spec")?;
        let output = oci.execute().context("OCI execute")?;
        Ok(output.to_string())
    }
}
