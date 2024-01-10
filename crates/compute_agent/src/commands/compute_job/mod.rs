use anyhow::Result;
use clap::Parser;
use service_config::ServiceConfig;

#[derive(Parser)]
pub struct ComputeJobOpts {
    #[clap(long)]
    cid: String,
}

// Should return the UUID of the job
pub async fn run(_opts: &ComputeJobOpts, _config: &ServiceConfig) -> Result<()> {
    // Connect to the server and request the job via stringified CID.
    // The server should queue the job, and return the job's ID (UUID).
    Ok(())
}
