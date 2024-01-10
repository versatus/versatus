use clap::Parser;

pub mod job_status;
pub mod queue_job;

#[derive(Parser)]
pub struct ComputeJobOpts {
    #[clap(long)]
    cid: String,
}
