use std::net::SocketAddr;

use events::{Event, EventPublisher, EventSubscriber};
use mempool::MempoolReadHandleFactory;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use tokio::task::JoinHandle;
use vrrb_config::NodeConfig;
use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};

use crate::result::{NodeError, Result};

pub async fn setup_rpc_api_server(
    config: &NodeConfig,
    events_tx: EventPublisher,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
    mut jsonrpc_events_rx: EventSubscriber,
) -> Result<(JoinHandle<Result<()>>, SocketAddr)> {
    let jsonrpc_server_config = JsonRpcServerConfig {
        address: config.jsonrpc_server_address,
        node_type: config.node_type,
        events_tx,
        vrrbdb_read_handle,
        mempool_read_handle_factory,
    };

    let (jsonrpc_server_handle, resolved_jsonrpc_server_addr) =
        JsonRpcServer::run(&jsonrpc_server_config)
            .await
            .map_err(|err| NodeError::Other(format!("unable to start JSON-RPC server: {err}")))?;

    let jsonrpc_server_handle = tokio::spawn(async move {
        if let Ok(evt) = jsonrpc_events_rx.recv().await {
            if let Event::Stop = evt.into() {
                jsonrpc_server_handle.stop().map_err(|err| {
                    NodeError::Other(format!("JSON-RPC event has stopped: {err}"))
                })?;
                return Ok(());
            }
        }

        Ok(())
    });

    info!(
        "JSON-RPC server started at {}",
        resolved_jsonrpc_server_addr
    );

    // let jsonrpc_server_handle = Some(jsonrpc_server_handle);

    Ok((jsonrpc_server_handle, resolved_jsonrpc_server_addr))
}
