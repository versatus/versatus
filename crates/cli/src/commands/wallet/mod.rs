mod get;
mod info;
mod new;
mod transfer;

use clap::{Parser, Subcommand};
use serde_json;

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
    Transfer {
        #[clap(short, long)]
        address_number: u32,
        #[clap(short, long)]
        to: String,
        #[clap(short, long)]
        amount: u128,
        #[clap(short, long)]
        token: Option<String>,
    },

    /// Create a new account on the network
    New {
        #[clap(short, long)]
        address: String,
        #[clap(short, long)]
        account: String,
    },

    /// Gets information about an account
    Get {
        #[clap(short, long)]
        address: String
    },
}


#[allow(unreachable_patterns)]
pub async fn exec(args: WalletOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    match sub_cmd {
        WalletCmd::Info => info::exec().await,
        WalletCmd::Transfer {
            address_number, to, amount, token 
        } => { 
                transfer::exec(address_number, to, amount, token).await?;

                Ok(())
        },
        WalletCmd::New { address, account } => {
            let address = if let Ok(addr) = serde_json::from_str(&address) {
                addr
            } else {
                return Err(CliError::Other("invalid address".to_string()));
            };

            let account = if let Ok(acct) = serde_json::from_str(&account) {
                acct
            } else {
                return Err(CliError::Other("invalid account".to_string()))
            };

            new::exec(address, account).await?;

            Ok(())
        },
        WalletCmd::Get { address } => {
            let address = if let Ok(addr) = serde_json::from_str(&address) {
                addr
            } else {
                return Err(CliError::Other("invalid address".to_string()));
            };

            if let Some(acct) = get::exec(address).await {
                println!("{:?}", acct);
            };

            Ok(())
        },
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}


