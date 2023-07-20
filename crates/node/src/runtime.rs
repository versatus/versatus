use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
};

use block::Block;
use bulldag::graph::BullDag;
use events::{Event, EventPublisher, EventRouter, EventSubscriber, DEFAULT_BUFFER};
use mempool::MempoolReadHandleFactory;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use tokio::task::JoinHandle;
use vrrb_config::NodeConfig;
use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};

use crate::{
    result::{NodeError, Result},
    RuntimeComponent,
    RuntimeComponents,
};

pub async fn setup_runtime_components(
    original_config: &NodeConfig,
    router: &EventRouter,
    events_tx: EventPublisher,
) -> Result<RuntimeComponents> {
    let mut config = original_config.clone();

    let mut mempool_events_rx = router.subscribe(None)?;
    let vrrbdb_events_rx = router.subscribe(None)?;
    let network_events_rx = router.subscribe(None)?;
    let controller_events_rx = router.subscribe(None)?;
    let miner_events_rx = router.subscribe(None)?;
    let farmer_events_rx = router.subscribe(None)?;
    let harvester_events_rx = router.subscribe(None)?;
    let jsonrpc_events_rx = router.subscribe(Some("json-rpc-api-control".into()))?;
    let dkg_events_rx = router.subscribe(None)?;
    let miner_election_events_rx = router.subscribe(None)?;
    let quorum_election_events_rx = router.subscribe(None)?;
    let indexer_events_rx = router.subscribe(None)?;
    let dag_events_rx = router.subscribe(None)?;
    let swarm_module_events_rx = router.subscribe(None)?;

    let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

    let runtime_components = RuntimeComponents {
        node_config: config,
        mempool_handle: None,
        state_handle: None,
        gossip_handle: None,
        //
        // TODO: re-enable these
        jsonrpc_server_handle: None,
        miner_handle: None,
        dkg_handle: None,
        miner_election_handle: None,
        quorum_election_handle: None,
        farmer_handle: None,
        harvester_handle: None,
        indexer_handle: None,
        dag_handle: None,
        raptor_handle: None,
        scheduler_handle: None,
        node_gui_handle: None,
    };

    Ok(runtime_components)
}

async fn setup_rpc_api_server(
    config: &NodeConfig,
    events_tx: EventPublisher,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
    mut jsonrpc_events_rx: EventSubscriber,
) -> Result<(Option<JoinHandle<Result<()>>>, SocketAddr)> {
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

    let jsonrpc_server_handle = Some(jsonrpc_server_handle);

    Ok((jsonrpc_server_handle, resolved_jsonrpc_server_addr))
}

fn _setup_reputation_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

fn _setup_credit_model_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}
