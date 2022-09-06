use telemetry::TelemetryError;

use node_cli::CliError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("unable to setup telemetry subscriber: {0}")]
    Telemetry(#[from] TelemetryError),

    #[error("cli error: {0}")]
    Cli(#[from] CliError),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Service is responsible for initializing the node, handling networking and config management
#[derive(Debug, Default)]
pub struct Service {}

impl Service {
    pub fn new() -> Self {
        Self::default()
    }

    #[tracing::instrument]
    pub async fn start(&self) -> Result<()> {
        // TODO: setup ports and control channels and loops
        // TODO import node and feed it the appropriate args

        telemetry::info!("starting Node service");

        telemetry::info!("exiting service");

        Ok(())
    }
}
