use clap::Parser;
use internal_rpc::job_queue::ServiceJobType;

pub mod job_status;
pub mod queue_job;

/// Options to pass to subcommands pertaining to compute jobs.
#[derive(Parser)]
pub struct ComputeJobOpts {
    /// A string representation of a compute job's CID.
    #[clap(long)]
    cid: String,

    /// The type of compute job to add to the job queue.
    #[clap(long, short = 'j')]
    job_type: Option<ServiceJobType>,

    /// The path to a JSON file that represents the inputs
    /// for a compute job.
    #[clap(long, short = 'p')]
    input_path: Option<String>,
}
