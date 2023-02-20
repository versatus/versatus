use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::Address;
use vrrb_core::account::Account;
use wallet::v2::Wallet;

use crate::result::CliError;

pub async fn exec(address: Address, account: Account) -> Result<(), CliError> {
    let rpc_server = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9293);

    let mut wallet = Wallet::new(rpc_server)
        .await
        .map_err(|err| CliError::Other("unable to create wallet".to_string()))?;

    wallet
        .create_account()
        .await
        .map_err(|err| CliError::Other("unable to create account in state".to_string()))?;

    Ok(())
}
