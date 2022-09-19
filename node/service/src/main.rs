use std::io;

use service::{Result, Service};

/// Main entrypoint
#[tokio::main]
async fn main() -> Result<()> {
    telemetry::TelemetrySubscriber::init(io::stdout)?;

    let _cli = node_cli::parse()?;

    let srv = Service::new();

    srv.start().await?;

    Ok(())
}
