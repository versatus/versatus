use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{Address, PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use vrrb_core::account::Account;
use wallet::v2::{Wallet, WalletConfig};

use crate::result::CliError;

pub async fn exec(address: Address, kp: (SecretKey, PublicKey)) -> Option<Account> {
    let rpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9293);

    let (secret_key, public_key) = kp;

    let wallet_config = WalletConfig {
        rpc_server_address,
        secret_key,
        public_key,
    };

    let res = Wallet::new(wallet_config)
        .await
        .map_err(|err| CliError::Other(err.to_string()));

    if let Ok(mut wallet) = res {
        let opt = wallet.get_account(address).await;
        return opt;
    }

    return None;
}
