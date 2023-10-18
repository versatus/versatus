pub mod config;
pub mod keygen;
pub mod node;
pub mod utils;
pub mod wallet;

use crate::{
    cli::{Args, Commands},
    result::{CliError, Result},
};

pub async fn exec(args: Args) -> Result<()> {
    telemetry::debug!("args: {:?}", args);

    let cmd = args.command;

    match cmd {
        Some(Commands::Node(node_args)) => node::exec(*node_args).await,
        Some(Commands::Wallet(wallet_args)) => wallet::exec(wallet_args).await,
        Some(Commands::Keygen(keygen_args)) => keygen::exec(keygen_args),
        None => Err(CliError::NoSubcommand),
        _ => Err(CliError::InvalidCommand(format!("{cmd:?}"))),
    }
}
