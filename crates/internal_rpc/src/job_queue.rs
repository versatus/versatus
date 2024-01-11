use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Debug, time::Instant};

pub trait ServiceJob: Send + Sync {
    fn new(cid: &str, kind: ServiceJobType) -> Self;
    fn cid(&self) -> String;
    fn kind(&self) -> ServiceJobType;
    fn inst(&self) -> Instant;
    fn status(&self) -> ServiceJobStatus;
    fn uptime(&self) -> u64 {
        self.inst().elapsed().as_secs()
    }
}

/// The status of a job.
// TODO(@eureka-cpu): Figure out how to add Instant to bypass serde
// so we can track how long each operation takes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ServiceJobStatus {
    /// Job is in queue
    Waiting,
    /// Job is in progress
    Running,
    /// Job is completed
    Complete,
}

// TODO(@eureka-cpu): It may be possible to make most of these operations O(1) time.
#[derive(Debug, Serialize, Deserialize)]
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
    /// Add a new job to the front of the queue.
    pub(crate) fn queue_job(&mut self, cid: &str, kind: ServiceJobType) {
        self.0.push_front(J::new(cid, kind));
    }
    /// Get the status of a job in queue. Takes O(n) time.
    /// Best case scenario, the job in question is the last job in queue,
    /// ie. it's the first item in the vec.
    pub(crate) fn job_status(&self, cid: &str) -> Option<ServiceJobStatus> {
        for job in self.0.iter() {
            if &job.cid() == cid {
                return Some(job.status());
            }
        }
        None
    }
    /// De-queue a job from the end of the queue once it is complete.
    /// This is only to be used by the automated process when a job completes.
    /// To remove a particular job manually, use the `kill_job` method.
    // This operation take O(1) time.
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
    // Takes O(n) time, but I don't realistically see this being
    // used often so the trade off isn't bad.
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
