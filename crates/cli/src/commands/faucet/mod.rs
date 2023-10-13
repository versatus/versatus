use std::{collections::HashMap, net::SocketAddr, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use primitives::Address;
use serde_json;
use faucet::faucet::{FaucetConfig, Faucet};
use vrrb_core::{account::Account, helpers::read_or_generate_keypair_file};
use vrrb_core::transactions::Token;
use wallet::v2::{AddressAlias, Wallet, WalletConfig};

use crate::result::{CliError, Result};

#[derive(Debug, Subcommand)]
pub enum FaucetCmd {
    /// Run a faucet with the provided configuration
    Run,
}

#[derive(Parser, Debug)]
pub struct FaucetOpts {
    #[clap(subcommand)]
    pub subcommand: FaucetCmd,

    #[clap(long, default_value = "127.0.0.1:9293")]
    pub rpc_server_address: SocketAddr,

    /// Secret key to use when signing transactions
    #[clap(long)]
    pub secret_key: String,

    /// Secret key to use when signing transactions
    #[clap(long, default_value = "9294")]
    pub host_port: String,
}

pub async fn exec(args: FaucetOpts) -> Result<()> {

    let sub_cmd = args.subcommand;

    match sub_cmd {
        FaucetCmd::Run => {
            let config = FaucetConfig {
                rpc_server_address: args.rpc_server_address,
                server_port: args.host_port.parse::<u16>().unwrap(),
                secret_key: args.secret_key,
                transfer_amount: 10,
            };

            let faucet = Faucet::new(config).await.expect("Failed to create faucet");

            faucet.start().await.map_err(|err| {
                CliError::Other(format!("Failed to start faucet: {}", err))
            })
        },
        _ => Err(CliError::InvalidCommand(format!("{sub_cmd:?}"))),
    }
}