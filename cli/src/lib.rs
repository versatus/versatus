use clap::Parser;

mod cli;
pub(crate) use cli::*;
pub(crate) mod commands;
pub mod result;

#[telemetry::instrument]
pub async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    commands::exec(args).await?;

    Ok(())
}
