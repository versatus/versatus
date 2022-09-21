use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid command {0} provided")]
    InvalidCommand(String),

    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] telemetry::TelemetryError),

    #[error("node runtime error: {0}")]
    NodeRuntime(#[from] runtime::result::RuntimeError),

    #[error("node error: {0}")]
    Node(#[from] node::result::NodeError),

    #[error("storage error: {0}")]
    Storage(#[from] storage::StorageError),
}

pub type Result<T> = std::result::Result<T, CliError>;
