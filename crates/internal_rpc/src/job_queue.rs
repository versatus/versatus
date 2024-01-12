use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fmt::Debug, time::Instant};

/// The API for interacting with queued [`ServiceJob`]s
/// in the [`ServiceJobQueue`]
pub trait ServiceJobApi: Send + Sync {
    /// Create a new service job
    fn new(cid: &str, uuid: uuid::Uuid, kind: ServiceJobType) -> Self;
    /// Return the job's CID
    fn cid(&self) -> String;
    /// Return the job's UUID
    fn uuid(&self) -> uuid::Uuid;
    /// Return the type of the job
    fn kind(&self) -> ServiceJobType;
    /// Return the [`Instant`] the job was spawned
    fn inst(&self) -> Instant;
    /// Return the status of the job
    fn status(&self) -> ServiceJobStatus;
    /// Return the uptime of the job in seconds
    fn uptime(&self) -> u64 {
        self.inst().elapsed().as_secs()
    }
}

/// A special struct used for implementing the `ServiceJobApi` to
/// get around certain types (`Instant`) not `implementing serde::{Serialize, Deserialize}`
#[derive(Debug)]
pub struct ServiceJob {
    cid: String,
    uuid: uuid::Uuid,
    kind: ServiceJobType,
    inst: Instant,
    status: ServiceJobStatus,
}
impl ServiceJobApi for ServiceJob {
    fn new(cid: &str, uuid: uuid::Uuid, kind: crate::job_queue::ServiceJobType) -> Self {
        Self {
            cid: cid.into(),
            uuid,
            kind,
            inst: Instant::now(),
            status: ServiceJobStatus::Waiting,
        }
    }
    fn cid(&self) -> String {
        self.cid.clone()
    }
    fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
    fn kind(&self) -> crate::job_queue::ServiceJobType {
        self.kind.clone()
    }
    fn inst(&self) -> std::time::Instant {
        self.inst
    }
    fn status(&self) -> ServiceJobStatus {
        self.status.clone()
    }
}

/// The status of a job.
// TODO(@eureka-cpu): Figure out how to add Instant to bypass serde
// UPDATE: you can use interfaces to get around serializing Instant directly
// as done with ServiceJob and ServiceJobApi
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
pub struct ServiceJobQueue<J: ServiceJobApi>(VecDeque<J>);
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
impl<J: ServiceJobApi + Debug> ServiceJobQueue<J> {
    pub(crate) fn new() -> Self {
        Self(VecDeque::new())
    }
    /// Add a new job to the front of the queue.
    pub(crate) fn queue_job(&mut self, cid: &str, kind: ServiceJobType) -> uuid::Uuid {
        let uuid = uuid::Uuid::new_v4();
        self.0.push_front(J::new(cid, uuid, kind));
        uuid
    }
    /// Get the status of a job in queue. Takes O(n) time.
    /// Best case scenario, the job in question is the last job in queue,
    /// ie. it's the first item in the vec.
    pub(crate) fn job_status(&self, uuid: uuid::Uuid) -> Option<ServiceJobStatus> {
        for job in self.0.iter() {
            if job.uuid() == uuid {
                return Some(job.status());
            }
        }
        None
    }
    /// De-queue a job from the end of the queue once it is complete.
    /// This is only to be used by the automated process when a job completes.
    /// To remove a particular job manually, use the `kill_job` method.
    // This operation take O(1) time.
    pub(crate) fn dequeue_job(&mut self, uuid_opt: Option<&uuid::Uuid>) -> Result<()> {
        if let Some(job) = self.0.pop_back() {
            if let Some(uuid) = uuid_opt {
                if &job.uuid() == uuid {
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
    pub(crate) fn kill_job(&mut self, uuid: &uuid::Uuid) -> Result<()> {
        for (pos, job) in self.0.iter().enumerate() {
            if &job.uuid() == uuid {
                self.0.remove(pos);
                break;
            }
        }
        Ok(())
    }
}
