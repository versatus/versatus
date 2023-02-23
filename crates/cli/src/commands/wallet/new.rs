use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{Address, PublicKey, SecretKey};
use secp256k1::{generate_keypair, hashes::sha256, rand, Message};
use vrrb_core::account::Account;
use wallet::v2::{Wallet, WalletConfig};

use crate::result::CliError;

pub async fn exec(
    rpc_server_address: SocketAddr,
    address: Address,
    account: Account,
    kp: (SecretKey, PublicKey),
) -> Result<(), CliError> {
    // TODO: read keypair from file
    let (secret_key, public_key) = generate_keypair(&mut rand::thread_rng());

    let wallet_config = WalletConfig {
        rpc_server_address,
        secret_key,
        public_key,
    };

    let mut wallet = Wallet::new(wallet_config)
        .await
        .map_err(|err| CliError::Other("unable to create wallet".to_string()))?;

    wallet
        .create_account()
        .await
        .map_err(|err| CliError::Other("unable to create account in state".to_string()))?;

    Ok(())
}
