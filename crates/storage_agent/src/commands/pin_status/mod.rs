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
    let pin_status: Result<()> = client
        .0
        .pinned_status(&opts.cid)
        .await
        .map_err(|err| err.into());
        match pin_status {
            Ok(()) => println!(
                "The content associated with CID '{}' is pinned in IPFS.",
                &opts.cid
            ),
            Err(e) => println!(
                "The content associated with CID '{}' is not pinned in IPFS. Details {}",
                &opts.cid,e
            ),
        }
    Ok(())
}
