//! A Versatus compute runtime for running a native payload under the Kontain runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities, JobSet};
use anyhow::{Context, Result};
use std::collections::HashMap;

const RUNTIME_DOMAINNAME: &str = "kontain";

/// A [ComputeRuntime] for executing a native compute payload within a Kontain Unikernel runtime.
pub struct KontainRuntime {}

impl ComputeRuntime for KontainRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Native
    }

    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    fn execute(&self, job_set: &JobSet, runtime_path: &str) -> Result<String> {
        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "unikernel+native".to_string());

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
        oci.execute().context("OCI execute")?;
        Ok("{ \"fake\": \"JSON payload\" }".to_string())
    }
}
