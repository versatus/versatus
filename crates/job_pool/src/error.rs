use thiserror::Error;

/// List of all possible errors related to JobPools .
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Error : {0}")]
    InvalidPoolWorkerConfig(String),

    #[error("Thread failed to complete its execution,Re run")]
    FailedToEndTask,
}
