mod get;
mod info;
mod new;
mod transfer;

use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use primitives::Address;
use secp256k1::{generate_keypair, rand};
use serde_json;
use vrrb_core::helpers::read_or_generate_keypair_file;

use crate::result::{CliError, Result};

#[derive(Parser, Debug)]
pub struct WalletOpts {
    #[clap(long, default_value = "127.0.0.1:9293")]
    pub rpc_server_address: SocketAddr,

    /// Secret key to use when signing transactions
    #[clap(long, default_value = "default")]
    pub identity: String,

    #[clap(subcommand)]
    pub subcommand: WalletCmd,
}

#[derive(Debug, Subcommand)]
pub enum WalletCmd {
    /// Get information about this wallet's configuration
    Info,

    /// Transfer objects between accounts
    Transfer {
        #[clap(long)]
        address_number: u32,
        #[clap(long)]
        to: String,
        #[clap(long)]
        amount: u128,
        #[clap(long)]
        token: Option<String>,
    },

    /// Create a new account on the network
    New {
        #[clap(long)]
        address: String,
        #[clap(long)]
        account: String,
    },

    /// Gets information about an account
    Get {
        #[clap(long)]
        address: String,
    },
}

pub async fn exec(args: WalletOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    let rpc_server_address = args.rpc_server_address;

    let data_dir = vrrb_core::storage_utils::get_wallet_data_dir()?.join("keys");

    std::fs::create_dir_all(&data_dir)?;

    let keypair_file_path = PathBuf::from(&data_dir).join(args.identity);

    let keypair = read_or_generate_keypair_file(&keypair_file_path)?;

    match sub_cmd {
        WalletCmd::Info => info::exec(rpc_server_address, keypair).await,
        WalletCmd::Transfer {
            address_number,
            to,
            amount,
            token,
        } => {
            transfer::exec(
                rpc_server_address,
                address_number,
                to,
                amount,
                token,
                keypair,
            )
            .await?;

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
                return Err(CliError::Other("invalid account".to_string()));
            };

            new::exec(rpc_server_address, address, account, keypair).await?;

            Ok(())
        },
        WalletCmd::Get { address } => {
            let address = Address::from_str(&address)?;

            if let Ok(account) = get::exec(rpc_server_address, address, keypair).await {
                let account_info = serde_json::to_string_pretty(&account)
                    .map_err(|err| CliError::Other(err.to_string()))?;

                println!("{}", account_info);
            };

            Ok(())
        },
        _ => Err(CliError::InvalidCommand(format!("{:?}", sub_cmd))),
    }
}
