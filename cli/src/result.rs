use runtime::RuntimeError;
use telemetry::Instrument;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid command {0} provided")]
    InvalidCommand(String),

    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] telemetry::TelemetryError),

    #[error("node runtime error: {0}")]
    NodeRuntime(#[from] runtime::RuntimeError),
}

pub type Result<T> = std::result::Result<T, CliError>;
