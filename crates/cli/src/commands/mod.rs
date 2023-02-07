use clap::AppSettings;

use crate::{
    cli::{Args, Commands},
    result::{CliError, Result},
};

pub mod node;

pub async fn exec(args: Args) -> Result<()> {
    telemetry::debug!("args: {:?}", args);

    let cmd = args.command;

    match cmd {
        Some(Commands::Node(node_args)) => node::exec(node_args).await,
        None => Err(CliError::NoSubcommand),
        _ => Err(CliError::InvalidCommand(format!("{:?}", cmd))),
    }
}
