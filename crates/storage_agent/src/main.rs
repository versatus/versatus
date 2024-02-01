mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use service_config::Config;
use telemetry::info;

static THIS_SERVICE_TYPE: &str = "storage";

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = cli::StorageCli::parse();
    env_logger::init();

    let service: String = match cli.service_type {
        Some(svc) => svc,
        None => THIS_SERVICE_TYPE.to_string(),
    };
    cli.service = "storage1".to_string();
    // Parse common services configuration
    let config = Config::from_file(&cli.config)?.find_service(&cli.service, &service)?;

    info!(
        "Matched service {}:{} to config: {:?}",
        cli.service, service, config
    );

    // Process subcommand
    match &cli.cmd {
        Some(cli::StorageCommands::Daemon(opts)) => {
            commands::daemon::run(opts, &config).await?;
        }
        Some(cli::StorageCommands::Status(opts)) => {
            commands::status::run(opts, &config).await?;
        }
        Some(cli::StorageCommands::RetrievalData(opts)) => {
            commands::data_retrieval::run(opts, &config).await?;
        }
        Some(cli::StorageCommands::PinObject(opts)) => {
            commands::pin_object::run(opts, &config).await?;
        }
        Some(cli::StorageCommands::CheckPinStatus(opts)) => {
            commands::pin_status::run(opts, &config).await?;
        }
        None => {}
    }

    Ok(())
}
