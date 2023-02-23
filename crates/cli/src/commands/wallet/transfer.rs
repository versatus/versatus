use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use vrrb_core::txn::{TransactionDigest, TxToken};
use wallet::v2::{Wallet, WalletConfig};

use crate::result::CliError;

pub async fn exec(
    rpc_server_address: SocketAddr,
    address_number: u32,
    to: String,
    amount: u128,
    token: Option<TxToken>,
    kp: (SecretKey, PublicKey),
) -> Result<TransactionDigest, CliError> {
    let rpc_server_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9293);

    let (secret_key, public_key) = kp;

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
        .map_err(|err| CliError::Other(err.to_string()))?;

    let timestamp = chrono::Utc::now().timestamp();

    // TODO: We need a faucet to first receive tokens from
    // or we need to initialize accounts with tokens on testnet
    wallet
        .send_transaction(address_number, to, amount, token, timestamp)
        .await
        .map_err(|err| CliError::Other(err.to_string()))
}
