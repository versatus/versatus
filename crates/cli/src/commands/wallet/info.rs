use crate::result::{Result, CliError};
use wallet::v2::Wallet;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

pub async fn exec() -> Result<()> {
    let rpc_server = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0,0,0,0)), 9293);

    let wallet: Wallet = Wallet::new(rpc_server).await.map_err(
        |err| CliError::Other(format!("unable to create wallet: {:?}", err).to_string())
    )?;
    
    println!("{:?}", wallet);
    Ok(())
}
