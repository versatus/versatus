use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use wallet::v2::{Wallet, WalletConfig};

use crate::result::{CliError, Result};

pub async fn exec(rpc_server_address: SocketAddr, kp: (SecretKey, PublicKey)) -> Result<()> {
    let (secret_key, public_key) = kp;

    let wallet_config = WalletConfig {
        rpc_server_address,
        secret_key,
        public_key,
    };

    let wallet = Wallet::new(wallet_config)
        .await
        .map_err(|err| CliError::Other(format!("unable to create wallet: {err}")))?;

    let wallet_info = wallet.info();
    let wallet_info = serde_json::to_string_pretty(&wallet_info)
        .map_err(|err| CliError::Other(format!("unable to serialize wallet information: {err}")))?;

    println!("{}", wallet_info);

    Ok(())
}
