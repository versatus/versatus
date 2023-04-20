use primitives::Address;
use vrrb_core::account::Account;
use wallet::v2::Wallet;

use crate::result::{CliError, Result};

pub async fn exec(wallet: &mut Wallet, address: Address) -> Result<Account> {
    let account = wallet
        .get_account(address)
        .await
        .map_err(|err| CliError::Other(err.to_string()))?;

    Ok(account)
}
