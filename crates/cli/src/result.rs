use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid command {0} provided")]
    InvalidCommand(String),

    #[error("no subcommand provided")]
    NoSubcommand,

    #[error("opts error: {0}")]
    OptsError(String),

    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] telemetry::custom_subscriber::TelemetryError),

    #[error("node error: {0}")]
    Node(#[from] node::result::NodeError),

    #[error("storage error: {0}")]
    Storage(#[from] vrrb_core::storage_utils::StorageError),

    #[error("primitive error: {0}")]
    Primitive(#[from] primitives::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("wallet error: {0}")]
    WalletError(#[from] wallet::v2::WalletError),

    #[error("core error: {0}")]
    CoreError(#[from] vrrb_core::result::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CliError>;
