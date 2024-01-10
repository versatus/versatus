use anyhow::Result;
use clap::Args;
use internal_rpc::{api::InternalRpcApiClient, client::InternalRpcClient};
use service_config::ServiceConfig;

/// Command line options structure for data retrieval subcommand
#[derive(Args, Debug)]
pub struct PinObjectOpts {
    cid: String,
    #[clap(short, long)]
    recursive: bool,
}

/// Make a data pinning RPC query against a running agent.
pub async fn run(opts: &PinObjectOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This where we would make the pin object  RPC call to the named service (global option) from
    // the service config file (global option) and show the result.
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
    let pinned_objects: Result<Vec<String>> = client
        .0
        .pin_object(&opts.cid, opts.recursive)
        .await
        .map_err(|err| err.into());

    match pinned_objects {
        Ok(objects) => {
            println!("Pinned Objects:");

            if objects.is_empty() {
                println!(
                    "No objects found for CID '{}' with{} recursive pinning.",
                    &opts.cid, opts.recursive
                );
            } else {
                for object in objects {
                    println!(" - {}", object);
                }
            }
        }
        Err(err) => {
            eprintln!("Error fetching pinned objects: {}", err);
            return Err(err);
        }
    }
    Ok(())
}
