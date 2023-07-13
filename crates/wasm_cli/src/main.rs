mod cli;
mod commands;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::WasmCli::parse();

    // Process subcommand
    match &cli.cmd {
        Some(cli::WasmCommands::Describe(opts)) => {
            commands::describe::run(&opts)?;
        },
        Some(cli::WasmCommands::Validate(opts)) => {
            commands::validate::run(&opts)?;
        },
        None => {},
    }

    Ok(())
}
