use anyhow::Result;
use clap::Parser;
use internal_rpc::{api::InternalRpcApiClient, client::InternalRpcClient};
use service_config::ServiceConfig;

/// Command line options structure for status subcommand
#[derive(Parser, Debug)]
pub struct StatusOpts {
    /// Check if the server is online.
    #[clap(long)]
    pub status: bool,
    /// Get a service response from the server.
    #[clap(long)]
    pub service_response: bool,
    /// Get the config that the server was built with.
    #[clap(long)]
    pub config: bool,
}

/// Make a status RPC query against a running agent.
pub async fn run(opts: &StatusOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This where we would make the status RPC call to the named service (global option) from
    // the service config file (global option) and show the result.
    let client = InternalRpcClient::new(&config.rpc_socket_addr()?).await?;

    let StatusOpts {
        status,
        service_response,
        config,
    } = *opts;
    if status {
        println!("{:?}", client.0.status().await);
    }
    if service_response {
        println!("{}", client.0.service_status_response().await?);
    }
    if config {
        println!("{:?}", client.0.service_config().await?);
    }

    Ok(())
}
