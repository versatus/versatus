use crate::commands::compute_job::ComputeJobOpts;
use anyhow::Result;
use internal_rpc::{api::InternalRpcApiClient, client::InternalRpcClient};
use service_config::ServiceConfig;

// Should return the status of the job
pub async fn run(opts: &ComputeJobOpts, config: &ServiceConfig) -> Result<()> {
    // Connect to the server and request the status of a job via stringified CID.
    // The server should return the job's status.
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;

    Ok(())
}
