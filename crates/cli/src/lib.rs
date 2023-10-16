use clap::Parser;

mod cli;
pub mod result;

pub(crate) use crate::cli::*;
pub mod commands;

#[telemetry::instrument]
pub async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    commands::exec(args).await?;

    Ok(())
}
