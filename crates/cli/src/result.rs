use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid command {0} provided")]
    InvalidCommand(String),

    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] telemetry::TelemetryError),

    #[error("node error: {0}")]
    Node(#[from] node::result::NodeError),

    #[error("storage error: {0}")]
    Storage(#[from] storage::StorageError),

    #[error("primitive error: {0}")]
    Primitive(#[from] primitives::types::node::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CliError>;
