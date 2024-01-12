use anyhow::Result;
use clap::Parser;
use internal_rpc::{api::InternalRpcApiClient, client::InternalRpcClient};
use service_config::ServiceConfig;

/// Command line options structure for status subcommand
#[derive(Parser, Debug)]
pub struct StatusOpts;

/// Make a status RPC query against a running agent.
pub async fn run(_opts: &StatusOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This where we would make the status RPC call to the named service (global option) from
    // the service config file (global option) and show the result.
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;

    println!("{}", client.0.status().await?);

    Ok(())
}
