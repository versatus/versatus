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
        },
        Some(cli::WasmCommands::Execute(opts)) => {
            commands::execute::run(opts)?;
        },
        Some(cli::WasmCommands::Validate(opts)) => {
            commands::validate::run(opts)?;
        },
        None => {},
    }

    Ok(())
}
