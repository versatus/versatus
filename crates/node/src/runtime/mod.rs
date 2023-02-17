// pub mod blockchain_module;
pub mod broadcast_module;
pub mod dkg_module;
pub mod farmer_harvester_module;
pub mod mempool_module;
pub mod mining_module;
pub mod state_module;
pub mod swarm_module;
pub mod validator_module;

use std::net::SocketAddr;

use network::network::BroadcastEngine;
pub use state_module::*;
use storage::vrrbdb::VrrbDbReadHandle;
use tokio::{
    sync::{broadcast::Receiver, mpsc::UnboundedSender},
    task::JoinHandle,
};
use vrrb_config::NodeConfig;
use vrrb_core::event_router::{DirectedEvent, Event, EventRouter, Topic};

use self::broadcast_module::{BroadcastModule, BroadcastModuleConfig};
use crate::{
    broadcast_controller::{BroadcastEngineController, BROADCAST_CONTROLLER_BUFFER_SIZE},
    NodeError,
};

pub async fn setup_runtime_components() {
    //
}

fn setup_event_routing_system() -> EventRouter {
    let mut event_router = EventRouter::new();
    event_router.add_topic(Topic::Control, Some(1));
    event_router.add_topic(Topic::State, Some(1));
    event_router.add_topic(Topic::Transactions, Some(100));
    event_router.add_topic(Topic::Network, Some(100));
    event_router.add_topic(Topic::Storage, Some(100));
    event_router.add_topic(Topic::Consensus, Some(100));

    event_router
}

async fn setup_gossip_network(
    config: &NodeConfig,
    events_tx: UnboundedSender<DirectedEvent>,
    mut network_events_rx: Receiver<Event>,
    mut controller_events_rx: Receiver<Event>,
    vrrbdb_read_handle: VrrbDbReadHandle,
) -> Result<(
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
    SocketAddr,
)> {
    let broadcast_module = BroadcastModule::new(BroadcastModuleConfig {
        events_tx: events_tx.clone(),
        vrrbdb_read_handle,
        udp_gossip_address_port: config.udp_gossip_address.port(),
        raptorq_gossip_address_port: config.raptorq_gossip_address.port(),
        node_type: config.node_type,
        node_id: config.id.as_bytes().to_vec(),
    })
    .await?;

    let addr = broadcast_module.local_addr();

    let (controller_tx, controller_rx) =
        tokio::sync::mpsc::channel::<Event>(BROADCAST_CONTROLLER_BUFFER_SIZE);

    let broadcast_engine = BroadcastEngine::new(config.udp_gossip_address.port(), 32)
        .await
        .map_err(|err| NodeError::Other(format!("unable to setup broadcast engine: {}", err)))?;

    let mut bcast_controller = BroadcastEngineController::new(broadcast_engine);

    // NOTE: starts the listening loop
    let broadcast_controller_handle = tokio::spawn(async move {
        bcast_controller
            .listen(controller_tx, controller_events_rx)
            .await
    });

    let mut broadcast_module_actor = ActorImpl::new(broadcast_module);

    let broadcast_handle = tokio::spawn(async move {
        broadcast_module_actor
            .start(&mut network_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok((
        Some(broadcast_handle),
        Some(broadcast_controller_handle),
        addr,
    ))
}

async fn setup_state_store(
    config: &NodeConfig,
    events_tx: UnboundedSender<DirectedEvent>,
    mut state_events_rx: Receiver<Event>,
) -> Result<(VrrbDbReadHandle, Option<JoinHandle<Result<()>>>)> {
    // TODO: restore state if exists

    let database_path = config.db_path();

    storage_utils::create_dir(database_path).map_err(|err| NodeError::Other(err.to_string()))?;

    let vrrbdb_config = VrrbDbConfig::default();

    let db = storage::vrrbdb::VrrbDb::new(vrrbdb_config);
    let vrrbdb_read_handle = db.read_handle();

    let mut state_module = StateModule::new(StateModuleConfig { events_tx, db });
    let mut state_module_actor = ActorImpl::new(state_module);

    let state_handle = tokio::spawn(async move {
        state_module_actor
            .start(&mut state_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok((vrrbdb_read_handle, Some(state_handle)))
}

async fn setup_rpc_api_server(
    config: &NodeConfig,
    events_tx: UnboundedSender<DirectedEvent>,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
) -> Result<(Option<JoinHandle<Result<()>>>, SocketAddr)> {
    let jsonrpc_server_config = JsonRpcServerConfig {
        address: config.jsonrpc_server_address,
        node_type: config.node_type,
        events_tx,
        vrrbdb_read_handle,
        mempool_read_handle_factory,
    };

    let resolved_jsonrpc_server_addr = JsonRpcServer::run(&jsonrpc_server_config)
        .await
        .map_err(|err| NodeError::Other(format!("unable to satrt JSON-RPC server: {}", err)))?;

    let jsonrpc_server_handle = Some(tokio::spawn(async { Ok(()) }));

    Ok((jsonrpc_server_handle, resolved_jsonrpc_server_addr))
}

fn setup_validation_module(
    events_tx: UnboundedSender<DirectedEvent>,
    mut validator_events_rx: Receiver<Event>,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let mut module = validator_module::ValidatorModule::new();

    let txn_validator_handle =
        tokio::spawn(async move { module.start(&mut validator_events_rx).await });

    Ok(Some(txn_validator_handle))
}

fn setup_mining_module(
    events_tx: UnboundedSender<DirectedEvent>,
    mut miner_events_rx: Receiver<Event>,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let mut module = mining_module::MiningModule::new();

    let miner_handle = tokio::spawn(async move { module.start(&mut miner_events_rx).await });

    Ok(Some(miner_handle))
}
