mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use service_config::Config;
use telemetry::info;

static THIS_SERVICE_TYPE: &str = "storage";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::StorageCli::parse();

    let service: String = match cli.service_type {
        Some(svc) => svc,
        None => THIS_SERVICE_TYPE.to_string(),
    };

    // Parse common services configuration
    let config = Config::from_file(&cli.config)?
        .find_service(&cli.service, &service)?;

    info!("Matched service {}:{} to config: {:?}", cli.service, service, config);

    // Process subcommand
    match &cli.cmd {
        Some(cli::StorageCommands::Daemon(opts)) => {
            commands::daemon::run(opts, &config).await?;
        },
        Some(cli::StorageCommands::Status(opts)) => {
            commands::status::run(opts, &config).await?;
        },
        None => {},
    }

    Ok(())
}
