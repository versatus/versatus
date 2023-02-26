use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::{PublicKey, SecretKey};
use secp256k1::{generate_keypair, rand};
use vrrb_core::txn::{Token, TransactionDigest};
use wallet::v2::{Wallet, WalletConfig};

use crate::result::CliError;

pub async fn exec(
    wallet: &mut Wallet,
    address_number: u32,
    to: String,
    amount: u128,
    token: Token,
) -> Result<TransactionDigest, CliError> {
    let timestamp = chrono::Utc::now().timestamp();

    // TODO: We need a faucet to first receive tokens from
    // or we need to initialize accounts with tokens on testnet
    let digest = wallet
        .send_transaction(address_number, to, amount, token, timestamp)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    Ok(digest)
}
