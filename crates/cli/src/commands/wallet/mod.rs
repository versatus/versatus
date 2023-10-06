mod get;
mod get_mempool;
mod info;
mod new;
mod transfer;

use std::{collections::HashMap, net::SocketAddr, path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use primitives::Address;
use serde_json;
use vrrb_core::{account::Account, helpers::read_or_generate_keypair_file};
use vrrb_core::transactions::Token;
use wallet::v2::{AddressAlias, Wallet, WalletConfig};

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
        // TODO: replace u32 with address aliases so they're easier to use
        from: AddressAlias,

        #[clap(long)]
        to: Address,

        #[clap(long)]
        amount: u128,

        #[clap(long)]
        token: Option<Token>,
    },

    /// Create a new account on the network
    New {
        #[clap(long)]
        alias: AddressAlias,
    },

    /// Gets information about an account
    Get {
        #[clap(long)]
        address: String,
    },

    /// Retrieves a snapshot of the values within mempool
    GetMempool {
        #[clap(long)]
        limit: Option<usize>,
    },
}

pub async fn exec(args: WalletOpts) -> Result<()> {
    let sub_cmd = args.subcommand;

    let rpc_server_address = args.rpc_server_address;

    let data_dir = vrrb_core::storage_utils::get_wallet_data_dir()?.join("keys");
    let accounts_data_dir = vrrb_core::storage_utils::get_wallet_data_dir()?
        .join("keys")
        .join("accounts");

    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&accounts_data_dir)?;

    // NOTE: master keypair
    let keypair_file_path = PathBuf::from(&data_dir).join(args.identity);

    let keypair = read_or_generate_keypair_file(&keypair_file_path)?;

    let (secret_key, public_key) = keypair;

    let (accounts, addresses) = restore_accounts_and_addresses(&accounts_data_dir)?;

    let wallet_config = WalletConfig {
        rpc_server_address,
        secret_key,
        public_key,
        accounts,
        addresses,
    };

    let mut wallet = Wallet::new(wallet_config)
        .await
        .map_err(|err| CliError::Other(format!("unable to create wallet: {err}")))?;

    match sub_cmd {
        WalletCmd::Info => info::exec(&wallet).await,
        WalletCmd::Transfer {
            from: address_number,
            to,
            amount,
            token,
        } => {
            let digest = transfer::exec(
                &mut wallet,
                address_number,
                to,
                amount,
                token.unwrap_or_default(),
            )
            .await?;

            println!("{digest}");

            Ok(())
        },
        WalletCmd::New { alias } => {
            new::exec(&mut wallet, &accounts_data_dir, alias).await?;

            Ok(())
        },
        WalletCmd::Get { address } => {
            let address = Address::from_str(&address)
                .map_err(|err| CliError::Other(err.to_string()))?;

            if let Ok(account) = get::exec(&mut wallet, address).await {
                let account_info = serde_json::to_string_pretty(&account)
                    .map_err(|err| CliError::Other(err.to_string()))?;

                println!("{account_info}");
            };

            Ok(())
        },
        WalletCmd::GetMempool { limit } => {
            get_mempool::exec(&mut wallet, limit).await?;

            Ok(())
        },
    }
}

fn restore_accounts_and_addresses(
    path: &PathBuf,
) -> Result<(HashMap<Address, Account>, HashMap<AddressAlias, Address>)> {
    let mut accounts = HashMap::new();
    let mut addresses = HashMap::new();

    let entries = std::fs::read_dir(path).map_err(|err| CliError::Other(err.to_string()))?;

    for entry in entries {
        let entry = entry.map_err(|err| CliError::Other(err.to_string()))?;
        let path = entry.path();

        let file_name = path
            .file_name()
            .ok_or(CliError::Other("unable to get file name".to_string()))
            .map_err(|err| CliError::Other(err.to_string()))?
            .to_str()
            .ok_or(CliError::Other("unable to get file name".to_string()))
            .map_err(|err| CliError::Other(err.to_string()))?;

        let alias =
            AddressAlias::from_str(file_name).map_err(|err| CliError::Other(err.to_string()))?;

        let account_string = std::fs::read_to_string(&path.join("account.json"))
            .map_err(|err| CliError::Other(err.to_string()))?;

        let account: Account = serde_json::from_str(&account_string)
            .map_err(|err| CliError::Other(err.to_string()))?;

        let (_, public) = read_or_generate_keypair_file(&path.join("keys"))
            .map_err(|err| CliError::Other(err.to_string()))?;

        let address = Address::new(public);

        accounts.insert(address.clone(), account);
        addresses.insert(alias, address.clone());
    }

    Ok((accounts, addresses))
}
