use telemetry::TelemetrySubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    TelemetrySubscriber::init(std::io::stdout)?;

    dbg!("IN MAIN");

    cli::run().await?;

    dbg!("DONE RUN IN MAIN");

    Ok(())
}
