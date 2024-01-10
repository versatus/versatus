mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::WasmCli::parse();

    env_logger::init();

    // Process subcommand
    match &cli.cmd {
        Some(cli::WasmCommands::Describe(opts)) => {
            commands::describe::run(opts)?;
        }
        Some(cli::WasmCommands::Execute(opts)) => {
            commands::execute::run(opts)?;
        }
        Some(cli::WasmCommands::Validate(opts)) => {
            commands::validate::run(opts)?;
        }
        Some(cli::WasmCommands::Publish(opts)) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async { commands::publish::run(opts).await })?;
        }
        None => {}
    }

    Ok(())
}
