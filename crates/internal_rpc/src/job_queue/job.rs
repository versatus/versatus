use serde::{Deserialize, Serialize};
use std::{fmt, time::Instant};

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
    /// Update the status of a job
    fn update_status(&mut self, state: ServiceJobState);
    /// Return the uptime of the job in seconds
    fn uptime(&self) -> u64 {
        self.inst().elapsed().as_secs()
    }
}

/// A special struct used for implementing the `ServiceJobApi` to
/// get around certain types (`Instant`) not `implementing serde::{Serialize, Deserialize}`
#[derive(Debug, Clone)]
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
    fn update_status(&mut self, state: ServiceJobState) {
        self.status.update(state)
    }
}

// Something like this probably already exists just using as placeholder for now
// to get something flowing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceJobType {
    Compute(ComputeJobExecutionType),
    Storage,
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

// TODO(@eureka-cpu): Impl Display and make the timestamps human-readable
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

/// The state of a job.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceJobState {
    /// Job was sent to the server
    #[default]
    Sent,
    /// Job was received by a [`ServiceReceiver`]
    Received,
    /// Job has started
    InProgress,
    /// Job is completed
    Complete,
}

/// The reponse sent from the server when requested by a client
/// about the state of a job.
#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceJobStatusResponse {
    pub(crate) status: ServiceJobState,
    pub(crate) uptime: u64,
}
