use std::io;

/// Main entrypoint for VRRB
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    telemetry::TelemetrySubscriber::init(io::stdout)?;

    node_cli::run().await?;

    Ok(())
}
