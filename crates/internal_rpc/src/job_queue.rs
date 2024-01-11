use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fmt::Debug,
    time::{Duration, Instant},
};

pub trait ServiceJob: Send + Sync {
    fn new(cid: &str, kind: ServiceJobType) -> Self;
    fn cid(&self) -> String;
    fn kind(&self) -> ServiceJobType;
    fn inst(&self) -> Instant;
    fn uptime(&self) -> u64 {
        self.inst().elapsed().as_secs()
    }
}

/// The status of a job.
pub enum ServiceJobStatus {
    /// Job is in progress, represents the uptime of the job.
    Running(Duration),
    /// Job is completed, represents the time the job took to execute.
    Complete(Duration),
}

#[derive(Debug)]
pub struct ServiceJobQueue<J: ServiceJob>(VecDeque<J>);
// Something like this probably already exists just using as placeholder for now
// to get something flowing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceJobType {
    Compute,
}
impl std::str::FromStr for ServiceJobType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "compute" => Ok(Self::Compute),
            _ => Err(anyhow::anyhow!("failed to parse string into job type")),
        }
    }
}
impl<J: ServiceJob + Debug> ServiceJobQueue<J> {
    pub(crate) fn new() -> Self {
        Self(VecDeque::new())
    }
    /// Add a new job to the queue.
    pub(crate) fn queue_job(&mut self, cid: &str, kind: ServiceJobType) {
        self.0.push_front(J::new(cid, kind));
    }
    /// De-queue a job once it is complete.
    pub(crate) fn dequeue_job(&mut self, cid_opt: Option<&str>) -> Result<()> {
        if let Some(job) = self.0.pop_back() {
            if let Some(cid) = cid_opt {
                if &job.cid() == cid {
                    println!("{job:?} completed in {}s", job.uptime());
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("given CID is not the current job"))
                }
            } else {
                println!("{job:?} completed in {}s", job.uptime());
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!("job queue is empty"))
        }
    }
    /// Remove a job from queue that is not the current job.
    pub(crate) fn kill_job(&mut self, cid: &str) -> Result<()> {
        for (pos, job) in self.0.iter().enumerate() {
            if &job.cid() == cid {
                self.0.remove(pos);
                break;
            }
        }
        Ok(())
    }
}
