use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::digest::TransactionDigest;
use vrrb_core::txn::TxToken;
use wallet::v2::Wallet;

use crate::result::CliError;

pub async fn exec(
    address_number: u32,
    to: String,
    amount: u128,
    token: Option<TxToken>,
) -> Result<TransactionDigest, CliError> {
    let rpc_server = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9293);

    let mut wallet = Wallet::new(rpc_server)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    wallet
        .create_account()
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    // TODO: We need a faucet to first receive tokens from
    // or we need to initialize accounts with tokens on testnet
    wallet
        .send_txn(address_number, to, amount, token)
        .await
        .map_err(|err| CliError::Other(err.to_string()))
}
