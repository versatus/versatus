use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use wallet::v2::{Wallet, WalletConfig};

use crate::result::{CliError, Result};

pub async fn exec(wallet: &Wallet) -> Result<()> {
    let wallet_info = wallet.info();
    let wallet_info = serde_json::to_string_pretty(&wallet_info)
        .map_err(|err| CliError::Other(format!("unable to serialize wallet information: {err}")))?;

    println!("{}", wallet_info);

    Ok(())
}
