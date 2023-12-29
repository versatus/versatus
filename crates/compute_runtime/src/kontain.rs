//! A Versatus compute runtime for running a native payload under the Kontain runtime.
use crate::oci::OciManagerBuilder;
use crate::runtime::{ComputeRuntime, ComputeRuntimeCapabilities};
use anyhow::{Context, Result};
use std::collections::HashMap;

const RUNTIME_DOMAINNAME: &str = "kontain";
const RUNTIME_PATH: &str = "/home/matthew/tmp/src/kontain-build/pkg/bin/crun";

/// A [ComputeRuntime] for executing a native compute payload within a Kontain Unikernel runtime.
pub struct KontainRuntime {}

impl ComputeRuntime for KontainRuntime {
    fn capabilities() -> ComputeRuntimeCapabilities {
        ComputeRuntimeCapabilities::Native
    }

    fn domainname() -> &'static str {
        RUNTIME_DOMAINNAME
    }

    fn setup(&self, job_id: &str, runtime_path: &str) -> Result<()> {
        let mut annotations: HashMap<String, String> = HashMap::new();
        annotations.insert("payload_type".to_string(), "unikernel+native".to_string());

        let mut oci = OciManagerBuilder::default()
            .runtime_path(runtime_path.to_string())
            .oci_runtime(RUNTIME_PATH.to_string())
            .container_payload(vec![
                "/bin/busybox".to_string(),
                "ls".to_string(),
                "-l".to_string(),
                "/".to_string(),
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
