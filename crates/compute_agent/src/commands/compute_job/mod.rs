use clap::Parser;

pub mod job_status;
pub mod queue_job;

/// Options to pass to subcommands pertaining to compute jobs.
#[derive(Parser)]
pub struct ComputeJobOpts {
    /// A string representation of a compute job's CID
    #[clap(long)]
    cid: String,

    /// The type of compute job to add to the job queue
    #[clap(long, short = 'j')]
    job_type: Option<ComputeJobType>,
}

/// The type of compute job. Used for adding to the job
/// queue or queries to the job queue to narrow search.
pub enum ComputeJobType {
    Unknown,
}
impl std::str::FromStr for ComputeJobType {
    type Err = anyhow::Error;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(ComputeJobType::Unknown)
    }
}
