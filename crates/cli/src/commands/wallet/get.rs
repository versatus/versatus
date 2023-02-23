use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{Address, PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use vrrb_core::account::Account;
use wallet::v2::{Wallet, WalletConfig};

use crate::result::{CliError, Result};

pub async fn exec(
    rpc_server_address: SocketAddr,
    address: Address,
    kp: (SecretKey, PublicKey),
) -> Result<Account> {
    let (secret_key, public_key) = kp;

    let wallet_config = WalletConfig {
        rpc_server_address,
        secret_key,
        public_key,
    };

    let mut wal = Wallet::new(wallet_config)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    let account = wal
        .get_account(address)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    Ok(account)
}
