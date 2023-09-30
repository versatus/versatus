use primitives::Address;
use vrrb_core::transactions::{RpcTransactionDigest, Token};
use wallet::v2::Wallet;

use crate::result::CliError;

pub async fn exec(
    wallet: &mut Wallet,
    address_number: u32,
    to: Address,
    amount: u128,
    token: Token,
) -> Result<RpcTransactionDigest, CliError> {
    let timestamp = chrono::Utc::now().timestamp();

    // TODO: We need a faucet to first receive tokens from
    // or we need to initialize accounts with tokens on testnet
    let digest = wallet
        .send_transaction(address_number, to, amount, token, timestamp)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    Ok(digest)
}
