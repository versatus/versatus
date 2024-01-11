use crate::commands::compute_job::ComputeJobOpts;
use anyhow::Result;
use internal_rpc::job_queue::{ServiceJob, ServiceJobStatus, ServiceJobType};
use service_config::ServiceConfig;
use std::time::Instant;

#[derive(Debug)]
pub struct ComputeJob {
    cid: String,
    kind: ServiceJobType,
    inst: Instant,
    status: ServiceJobStatus,
}
impl ServiceJob for ComputeJob {
    fn new(cid: &str, kind: ServiceJobType) -> Self {
        Self {
            cid: cid.into(),
            kind,
            inst: Instant::now(),
            status: ServiceJobStatus::Waiting,
        }
    }
    fn cid(&self) -> String {
        self.cid.clone()
    }
    fn kind(&self) -> ServiceJobType {
        self.kind.clone()
    }
    fn inst(&self) -> Instant {
        self.inst
    }
    fn status(&self) -> ServiceJobStatus {
        self.status.clone()
    }
}

// Should return the UUID of the job
pub async fn run(_opts: &ComputeJobOpts, _config: &ServiceConfig) -> Result<()> {
    // Connect to the server and request the job via stringified CID.
    // The server should queue the job, and return the job's ID (UUID).
    Ok(())
}
