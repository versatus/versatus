mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::WasmCli::parse();

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
        Some(cli::WasmCommands::TestInitDB(opts)) => {
            commands::testinitdb::run(opts)?;
        }
        Some(cli::WasmCommands::TestBalance(opts)) => {
            commands::testbalance::run(opts)?;
        }
        Some(cli::WasmCommands::TestContract(opts)) => {
            commands::testcontract::run(opts)?;
        }
        Some(cli::WasmCommands::Publish(opts)) => {
            let rt = tokio::runtime::Runtime::new()?;
            let _ = rt.block_on(async { commands::publish::run(opts).await })?;
        }
        None => {}
    }

    Ok(())
}
