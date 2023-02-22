use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use wallet::v2::{Wallet, WalletConfig};

use crate::result::{CliError, Result};

pub async fn exec(kp: (SecretKey, PublicKey)) -> Result<()> {
    let rpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9293);

    let (secret_key, public_key) = kp;

    let wallet_config = WalletConfig {
        rpc_server_address,
        secret_key,
        public_key,
    };

    let wallet = Wallet::new(wallet_config)
        .await
        .map_err(|err| CliError::Other(format!("unable to create wallet: {err}")))?;

    println!("{:?}", wallet);

    Ok(())
}
