use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid command {0} provided")]
    InvalidCommand(String),

    #[error("no subcommand provided")]
    NoSubcommand,

    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] telemetry::TelemetryError),

    #[error("node error: {0}")]
    Node(#[from] node::result::NodeError),

    #[error("storage error: {0}")]
    Storage(#[from] vrrb_core::storage_utils::StorageError),

    #[error("primitive error: {0}")]
    Primitive(#[from] primitives::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CliError>;
