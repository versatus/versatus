use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, Condvar, Mutex},
    time::Instant,
};

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
    fn status(&self) -> ServiceJobStatusResponse;
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
    fn new(cid: &str, uuid: uuid::Uuid, kind: ServiceJobType) -> Self {
        Self {
            cid: cid.into(),
            uuid,
            kind,
            inst: Instant::now(),
            status: Default::default(),
        }
    }
    fn cid(&self) -> String {
        self.cid.clone()
    }
    fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
    fn kind(&self) -> ServiceJobType {
        self.kind.clone()
    }
    fn inst(&self) -> std::time::Instant {
        self.inst
    }
    fn status(&self) -> ServiceJobStatusResponse {
        self.status.report()
    }
}

/// The state of a job.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceJobState {
    /// Job is in queue
    #[default]
    Waiting,
    /// Job is in progress
    Running,
    /// Job is completed
    Complete,
}
/// The state of a job and the last time the state was updated.
#[derive(Clone, Debug, PartialEq)]
pub struct ServiceJobStatus {
    state: ServiceJobState,
    timestamp: Instant,
}
impl Default for ServiceJobStatus {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            state: Default::default(),
        }
    }
}
impl ServiceJobStatus {
    /// Report the status of a job.
    pub fn report(&self) -> ServiceJobStatusResponse {
        ServiceJobStatusResponse {
            status: self.state.clone(),
            uptime: self.timestamp.elapsed().as_secs(),
        }
    }
    /// Update the status of a job.
    pub fn update(&mut self, state: ServiceJobState) {
        self.state = state;
        self.timestamp = Instant::now();
    }
}
/// The reponse sent from the server when requested by a client
/// about the state of a job.
#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceJobStatusResponse {
    pub(crate) status: ServiceJobState,
    pub(crate) uptime: u64,
}

// TODO(@eureka-cpu): It may be possible to make most of these operations O(1) time.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceJobQueue<J: ServiceJobApi> {
    pub(crate) queue: VecDeque<J>,
}
#[derive(Debug)]
pub struct Transmitter<J: ServiceJobApi> {
    store: Arc<Mutex<ServiceJobQueue<J>>>,
    emitter: Arc<Condvar>,
}
pub trait ServiceTransmitter<J: ServiceJobApi>: Send + Sync {
    fn send(&self, cid: &str, kind: ServiceJobType) -> uuid::Uuid;
    fn new(store: &Arc<Mutex<ServiceJobQueue<J>>>, emitter: &Arc<Condvar>) -> Self;
}
impl<J: ServiceJobApi> ServiceTransmitter<J> for Transmitter<J> {
    // TODO(@eureka-cpu): Use if let and return an Option<Uuid>
    /// Add a new job to the front of the queue.
    fn send(&self, cid: &str, kind: ServiceJobType) -> uuid::Uuid {
        let uuid = uuid::Uuid::new_v4();
        self.store
            .lock()
            .expect("failed to get lock for queue")
            .queue
            .push_front(J::new(cid, uuid, kind));
        self.emitter.notify_one();

        uuid
    }
    fn new(store: &Arc<Mutex<ServiceJobQueue<J>>>, emitter: &Arc<Condvar>) -> Self {
        Self {
            store: Arc::clone(store),
            emitter: Arc::clone(emitter),
        }
    }
}
#[derive(Debug)]
pub struct Receiver<J: ServiceJobApi> {
    store: Arc<Mutex<ServiceJobQueue<J>>>,
    emitter: Arc<Condvar>,
}
pub trait ServiceReceiver<J: ServiceJobApi>: Send + Sync {
    fn recv(&self) -> Option<J>;
    fn new(store: &Arc<Mutex<ServiceJobQueue<J>>>, emitter: &Arc<Condvar>) -> Self;
}
impl<J: ServiceJobApi> ServiceReceiver<J> for Receiver<J> {
    /// Wait for a job to be queued by a client.
    fn recv(&self) -> Option<J> {
        let mut store = self.store.lock().unwrap();

        while store.queue.is_empty() {
            store = self.emitter.wait(store).unwrap();
        }

        store.queue.pop_back()
    }
    fn new(store: &Arc<Mutex<ServiceJobQueue<J>>>, emitter: &Arc<Condvar>) -> Self {
        Self {
            store: Arc::clone(store),
            emitter: Arc::clone(emitter),
        }
    }
}
#[derive(Debug)]
pub(crate) struct ServiceQueueChannel<
    T: ServiceTransmitter<J>,
    R: ServiceReceiver<J>,
    J: ServiceJobApi,
> {
    pub(crate) tx: T,
    pub(crate) rx: R,
    marker: std::marker::PhantomData<J>,
}
impl<T: ServiceTransmitter<J>, R: ServiceReceiver<J>, J: ServiceJobApi + std::fmt::Debug>
    ServiceQueueChannel<T, R, J>
{
    pub(crate) fn new() -> Self {
        let store = Arc::new(Mutex::new(ServiceJobQueue::new()));
        let emitter = Arc::new(Condvar::new());

        Self {
            tx: T::new(&store, &emitter),
            rx: R::new(&store, &emitter),
            marker: std::marker::PhantomData,
        }
    }
}
// Something like this probably already exists just using as placeholder for now
// to get something flowing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceJobType {
    Compute(ComputeJobExecutionType),
}
/// The type of job we're intending to execute.
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ComputeJobExecutionType {
    /// A Smart Contract job requiring a runtime capable of assembling JSON input and executing
    /// WASM.
    SmartContract,
    /// An ad-hoc execution job. Always local to a node, and primarily used for
    /// testing/development.
    AdHoc,
    /// A null job type primarily used for internal/unit testing and runs nothing.
    Null,
}

impl fmt::Display for ComputeJobExecutionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::SmartContract => {
                write!(f, "Smart Contract")
            }
            Self::AdHoc => {
                write!(f, "Ad Hoc Task")
            }
            Self::Null => {
                write!(f, "Null Task")
            }
        }
    }
}
impl std::str::FromStr for ComputeJobExecutionType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let job = match s.trim().to_lowercase().as_str() {
            "contract" | "smart-contract" => ComputeJobExecutionType::SmartContract,
            "adhoc" | "ad-hoc" => ComputeJobExecutionType::AdHoc,
            "null" => ComputeJobExecutionType::Null,
            _ => {
                return Err(anyhow::anyhow!(
                    "failed to parse compute job type from string"
                ));
            }
        };
        Ok(job)
    }
}
impl<J: ServiceJobApi + fmt::Debug> ServiceJobQueue<J> {
    pub(crate) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
    /// Add a new job to the front of the queue.
    pub(crate) fn queue_job(&mut self, cid: &str, kind: ServiceJobType) -> uuid::Uuid {
        let uuid = uuid::Uuid::new_v4();
        self.queue.push_front(J::new(cid, uuid, kind));
        uuid
    }
    /// Get the status of a job in queue. Takes O(n) time.
    /// Best case scenario, the job in question is the last job in queue,
    /// ie. it's the first item in the vec.
    pub(crate) fn job_status(&self, uuid: uuid::Uuid) -> Option<ServiceJobStatusResponse> {
        for job in self.queue.iter() {
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
        if let Some(job) = self.queue.pop_back() {
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
        for (pos, job) in self.queue.iter().enumerate() {
            if &job.uuid() == uuid {
                self.queue.remove(pos);
                break;
            }
        }
        Ok(())
    }
}
