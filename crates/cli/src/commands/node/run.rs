use std::{net::SocketAddr, path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use config::{Config, ConfigError, File};
use hbbft::crypto::{serde_impl::SerdeSecret, PublicKey, SecretKey};
use node::Node;
use primitives::DEFAULT_VRRB_DATA_DIR_PATH;
use secp256k1::{rand, Secp256k1};
use serde::Deserialize;
use telemetry::{error, info};
use uuid::Uuid;
use vrrb_config::NodeConfig;
use vrrb_core::{
    event_router::Event,
    keypair::{self, read_keypair_file, write_keypair_file, Keypair},
};

use crate::{
    commands::node::RunOpts,
    result::{CliError, Result},
};

/// Configures and runs a VRRB Node
pub async fn run(args: RunOpts) -> Result<()> {
    let data_dir = storage::get_node_data_dir()?;
    let keypair_file_path = PathBuf::from(&data_dir).join("keypair");
    let keypair = match read_keypair_file(&keypair_file_path) {
        Ok(keypair) => keypair,
        Err(err) => {
            error!("Failed to read keypair file: {}", err);
            info!("Generating new keypair");
            let keypair = Keypair::random();

            write_keypair_file(&keypair, &keypair_file_path)
                .map_err(|err| CliError::Other(format!("failed to write keypair file: {err}")))?;

            keypair
        },
    };

    let mut node_config = NodeConfig::from(args.clone());
    node_config.keypair = keypair;

    if args.debug_config {
        dbg!(&node_config);
    }

    if args.dettached {
        run_dettached(node_config).await
    } else {
        run_blocking(node_config).await
    }
}

#[telemetry::instrument]
async fn run_blocking(node_config: NodeConfig) -> Result<()> {
    let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let vrrb_node = Node::start(&node_config, ctrl_rx)
        .await
        .map_err(|err| CliError::Other(String::from("failed to listen for ctrl+c")))?;

    let node_type = vrrb_node.node_type();

    info!("running {node_type:?} node in blocking mode");

    let node_handle = tokio::spawn(async move {
        // NOTE: starts the main node service
        vrrb_node.wait().await
    });

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| CliError::Other(format!("failed to listen for ctrl+c: {err}")))?;

    ctrl_tx
        .send(Event::Stop)
        .map_err(|err| CliError::Other(format!("failed to send stop event to node: {err}")))?;

    node_handle
        .await
        .map_err(|err| CliError::Other(format!("failed to join node task handle: {err}")))?;

    info!("node stopped");

    Ok(())
}

#[telemetry::instrument]
async fn run_dettached(node_config: NodeConfig) -> Result<()> {
    info!("running node in dettached mode");
    // start child process, run node within it
    Ok(())
}
