use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use primitives::Address;
use vrrb_core::account::Account;
use wallet::v2::Wallet;

use crate::result::CliError;

pub async fn exec(address: Address) -> Option<Account> {
    let rpc_server = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9293);

    let res = Wallet::new(rpc_server)
        .await
        .map_err(|err| CliError::Other(err.to_string()));

    if let Ok(mut wallet) = res {
        let opt = wallet.get_account(address).await;
        return opt;
    }

    return None;
}
