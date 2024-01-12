use crate::commands::compute_job::ComputeJobOpts;
use anyhow::Result;
use internal_rpc::{
    api::InternalRpcApiClient, client::InternalRpcClient, job_queue::ServiceJobType,
};
use service_config::ServiceConfig;

/// Add a compute job to the server's job queue.
pub async fn run(opts: &ComputeJobOpts, config: &ServiceConfig) -> Result<uuid::Uuid> {
    // Connect to the server and request the job via stringified CID.
    // The server should queue the job, and return the job's ID (UUID).
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
    let job_uuid = client
        .0
        .queue_job(&opts.cid, ServiceJobType::Compute(opts.job_type.to_owned()))
        .await?;
    println!("job UUID: {job_uuid:?}");

    Ok(job_uuid)
}
