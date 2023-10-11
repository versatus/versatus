pub mod config;
pub mod node;
pub(crate) mod utils;
pub mod wallet;
pub mod faucet;

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
        Some(Commands::Faucet(faucet_args)) => faucet::exec(faucet_args).await,
        None => Err(CliError::NoSubcommand),
        _ => Err(CliError::InvalidCommand(format!("{cmd:?}"))),
    }
}
