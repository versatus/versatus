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
        Some(cli::WasmCommands::InitTestDB(opts)) => {
            commands::inittestdb::run(opts)?;
        }
        Some(cli::WasmCommands::AssertBalance(opts)) => {
            commands::assertbalance::run(opts)?;
        }
        Some(cli::WasmCommands::Test(opts)) => {
            commands::test::run(opts)?;
        }
        None => {}
    }

    Ok(())
}
