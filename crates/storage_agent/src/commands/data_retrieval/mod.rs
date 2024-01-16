use anyhow::{Context, Result};
use clap::Args;
use internal_rpc::{
    api::{IPFSDataType, InternalRpcApiClient},
    client::InternalRpcClient,
};
use service_config::ServiceConfig;
use std::io::Write;
use telemetry::error;

/// Command line options structure for data retrieval subcommand
#[derive(Args, Debug)]
pub struct DataRetrievalOpts {
    cid: String,
    data_type: IPFSDataType,
    blob_path: String,
}

/// Make a data retrieval RPC query against a running agent.
pub async fn run(opts: &DataRetrievalOpts, config: &ServiceConfig) -> Result<()> {
    // XXX: This where we would make the get data  RPC call to the named service (global option) from
    // the service config file (global option) and show the result.
    let client = InternalRpcClient::new(config.rpc_socket_addr()?).await?;
    if let Ok(metadata) = std::fs::metadata(&opts.blob_path) {
        if !metadata.is_dir() {
            error!("Path exists but is not a directory.");
            return Err(anyhow::anyhow!(
                "Path exists but is not a directory.".to_string()
            ));
        }
    } else if !opts.blob_path.is_empty() {
        error!("Path does not exist.");
        return Err(anyhow::anyhow!("Path does not exist.".to_string()));
    }
    let blob = client.0.get_data(&opts.cid, opts.data_type).await?;
    if !blob.is_empty() {
        for (cid, blob) in blob.iter() {
            let file_path = std::path::Path::new(&opts.blob_path).join(cid);
            let mut file = std::fs::File::create(&file_path)
                .with_context(|| format!("Failed to create/open file at {:?}", &file_path))?;
            file.write_all(blob)
                .with_context(|| format!("Failed to write data to file at {:?}", &file_path))?;

            file.sync_all()
                .with_context(|| format!("Failed to sync data to disk at {:?}", &file_path))?;
        }
    };

    Ok(())
}
