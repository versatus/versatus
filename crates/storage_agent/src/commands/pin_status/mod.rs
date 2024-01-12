use anyhow::Result;
use clap::Args;
use internal_rpc::{api::InternalRpcApiClient, client::InternalRpcClient};
use service_config::ServiceConfig;

/// Command line options structure for data retrieval subcommand
#[derive(Args, Debug)]
pub struct PinStatusOpts {
    cid: String,
}

/// Make a checking pinning status RPC query against a running agent.
pub async fn run(opts: &PinStatusOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This where we would make the pin object  RPC call to the named service (global option) from
    // the service config file (global option) and show the result.
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
    let pin_status: Result<bool> = client
        .0
        .is_pinned(&opts.cid)
        .await
        .map_err(|err| err.into());
    if let Ok(pin_status) = pin_status {
        match pin_status {
            true => println!(
                "The content associated with CID '{}' is pinned in IPFS.",
                &opts.cid
            ),
            false => println!(
                "The content associated with CID '{}' is not pinned in IPFS.",
                &opts.cid
            ),
        }
    }
    Ok(())
}
