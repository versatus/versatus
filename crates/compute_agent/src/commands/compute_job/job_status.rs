use crate::commands::compute_job::ComputeJobOpts;
use anyhow::Result;
use service_config::ServiceConfig;

pub enum ComputeJobStatus {
    /// Job is in progress, represents the uptime of the job.
    Running(std::time::Duration),
    /// Job is completed, represents the time the job took to execute.
    Complete(std::time::Duration),
}

// Should return the status of the job
pub async fn run(_opts: &ComputeJobOpts, _config: &ServiceConfig) -> Result<ComputeJobStatus> {
    // Connect to the server and request the status of a job via stringified CID.
    // The server should return the job's status.
    Ok(ComputeJobStatus::Complete(std::time::Duration::from_secs(
        0,
    )))
}
