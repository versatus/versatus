use clap::{Parser, Subcommand};
use std::path::PathBuf;
use telemetry::Instrument;

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
