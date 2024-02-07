use crate::commands::compute_job::ComputeJobOpts;
use anyhow::Result;
use internal_rpc::{api::InternalRpcApiClient, client::InternalRpcClient};
use service_config::ServiceConfig;

/// Return the status of a compute job.
pub async fn run(opts: &ComputeJobOpts, config: &ServiceConfig) -> Result<()> {
    // Connect to the server and request the status of a job via stringified CID.
    // The server should return the job's status.
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
    if let Some(job_status) = client.0.job_status(opts.uuid).await? {
        println!("Status of job '{}': {job_status:?}", opts.uuid);
    } else {
        println!("No active job with UUID: {}", opts.uuid);
    }

    Ok(())
}
