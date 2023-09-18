use anyhow::Result;
use clap::Parser;
use service_config::ServiceConfig;

/// Structure representing command line options to the daemon subcommand
#[derive(Parser, Debug)]
pub struct DaemonOpts {
}

/// Start the Compute Agent Daemon
pub async fn run(_opts: &DaemonOpts, _config: &ServiceConfig) -> Result<()> {
    // XXX: This is where we should start the RPC server listener and process incoming requests
    // using the service name and service config provided in the global command line options.
    Ok(())
}
