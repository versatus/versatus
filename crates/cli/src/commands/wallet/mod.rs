mod get;
mod info;
mod new;
mod transfer;

use clap::{Parser, Subcommand};

use crate::result::{CliError, Result};

#[derive(Parser, Debug)]
pub struct WalletOpts {
    #[clap(subcommand)]
    pub subcommand: WalletCmd,
}

#[derive(Debug, Subcommand)]
pub enum WalletCmd {
    /// Get information about this wallet's configuration
    Info,

    /// Transfer objects between accounts
    Transfer,

    /// Create a new account on the network
    New,

    /// Gets information about an account
    Get,
}

pub async fn exec(args: WalletOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    match sub_cmd {
        WalletCmd::Info => info::exec().await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}
