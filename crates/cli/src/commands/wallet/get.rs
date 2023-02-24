use std::net::SocketAddr;

use primitives::{Address, PublicKey, SecretKey};
use secp256k1::hashes::serde_impl;
use vrrb_core::account::Account;
use wallet::v2::{Wallet, WalletConfig};

use crate::result::{CliError, Result};

pub async fn exec(wallet: &mut Wallet, address: Address) -> Result<Account> {
    let account = wallet
        .get_account(address)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    Ok(account)
}
