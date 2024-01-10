use clap::Parser;

pub mod job_status;
pub mod queue_job;

#[derive(Parser)]
pub struct ComputeJobOpts {
    #[clap(long)]
    cid: String,
    #[clap(long, short = 'j')]
    job_type: Option<ComputeJobType>,
}

pub enum ComputeJobType {
    Unknown,
}
impl std::str::FromStr for ComputeJobType {
    type Err = anyhow::Error;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(ComputeJobType::Unknown)
    }
}
