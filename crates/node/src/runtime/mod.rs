#![allow(unused_imports)]
use std::net::SocketAddr;

use events::{DirectedEvent, Event, EventRouter, Topic};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use miner::MinerConfig;
use network::network::BroadcastEngine;
use primitives::Address;
use storage::{
    storage_utils,
    vrrbdb::{VrrbDbConfig, VrrbDbReadHandle},
};
use telemetry::info;
use theater::{Actor, ActorImpl};
use tokio::{
    sync::{broadcast::Receiver, mpsc::UnboundedSender},
    task::JoinHandle,
};
use vrrb_config::NodeConfig;
use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};

use self::{
    broadcast_module::{BroadcastModule, BroadcastModuleConfig},
    mempool_module::{MempoolModule, MempoolModuleConfig},
    mining_module::{MiningModule, MiningModuleConfig},
    state_module::StateModule,
    validator_module::ValidatorModule,
};
use crate::{
    broadcast_controller::{BroadcastEngineController, BROADCAST_CONTROLLER_BUFFER_SIZE},
    NodeError,
    Result,
    RuntimeModule,
};

pub mod broadcast_module;
pub mod credit_model_module;
pub mod dkg_module;
pub mod farmer_harvester_module;
pub mod farmer_module;
pub mod harvester_module;
pub mod mempool_module;
pub mod mining_module;
pub mod reputation_module;
pub mod state_module;
pub mod swarm_module;
pub mod validator_module;

pub async fn setup_runtime_components(
    original_config: &NodeConfig,
    events_tx: UnboundedSender<(Topic, Event)>,
    mut mempool_events_rx: Receiver<Event>,
    vrrbdb_events_rx: Receiver<Event>,
    network_events_rx: Receiver<Event>,
    controller_events_rx: Receiver<Event>,
    validator_events_rx: Receiver<Event>,
    miner_events_rx: Receiver<Event>,
    jsonrpc_events_rx: Receiver<Event>,
) -> Result<(
    NodeConfig,
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<Result<()>>>,
)> {
    let mut config = original_config.clone();

    let mempool = LeftRightMempool::new();
    let mempool_read_handle_factory = mempool.factory();

    let mempool_module = MempoolModule::new(MempoolModuleConfig {
        mempool,
        events_tx: events_tx.clone(),
    });

    let mut mempool_module_actor = ActorImpl::new(mempool_module);

    let mempool_handle = tokio::spawn(async move {
        mempool_module_actor
            .start(&mut mempool_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    let mempool_handle = Some(mempool_handle);

    let (state_read_handle, state_handle) = setup_state_store(
        &config,
        events_tx.clone(),
        vrrbdb_events_rx,
        mempool_read_handle_factory.clone(),
    )
    .await?;

    let mut gossip_handle = None;
    let mut broadcast_controller_handle = None;

    if !config.disable_networking {
        let (new_gossip_handle, new_broadcast_controller_handle, gossip_addr) =
            setup_gossip_network(
                &config,
                events_tx.clone(),
                network_events_rx,
                controller_events_rx,
                state_read_handle.clone(),
            )
            .await?;

        gossip_handle = new_gossip_handle;
        broadcast_controller_handle = new_broadcast_controller_handle;
        config.udp_gossip_address = gossip_addr;
    }

    let (jsonrpc_server_handle, resolved_jsonrpc_server_addr) = setup_rpc_api_server(
        &config,
        events_tx.clone(),
        state_read_handle.clone(),
        mempool_read_handle_factory.clone(),
        jsonrpc_events_rx,
    )
    .await?;

    config.jsonrpc_server_address = resolved_jsonrpc_server_addr;

    info!("JSON-RPC server address: {}", config.jsonrpc_server_address);

    // TODO: make nodes start with some preconfigured state
    let txn_validator_handle = setup_validation_module(
        events_tx.clone(),
        validator_events_rx,
        mempool_read_handle_factory.clone(),
    )?;

    let miner_handle = setup_mining_module(
        &config,
        events_tx.clone(),
        state_read_handle.clone(),
        mempool_read_handle_factory.clone(),
        miner_events_rx,
    )?;

    Ok((
        config,
        mempool_handle,
        state_handle,
        gossip_handle,
        broadcast_controller_handle,
        jsonrpc_server_handle,
        txn_validator_handle,
        miner_handle,
    ))
}

#[allow(unused)]
fn setup_event_routing_system() -> EventRouter {
    let mut event_router = EventRouter::new();
    event_router.add_topic(Topic::Control, Some(1));
    event_router.add_topic(Topic::State, Some(1));
    event_router.add_topic(Topic::Network, Some(100));
    event_router.add_topic(Topic::Storage, Some(100));
    event_router.add_topic(Topic::Consensus, Some(100));

    event_router
}

#[allow(unused_mut)]
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

    #[allow(unused)]
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

#[allow(unused)]
async fn setup_state_store(
    config: &NodeConfig,
    events_tx: UnboundedSender<DirectedEvent>,
    mut state_events_rx: Receiver<Event>,
    mempool_read_handle_factory: MempoolReadHandleFactory,
) -> Result<(VrrbDbReadHandle, Option<JoinHandle<Result<()>>>)> {
    let mut vrrbdb_config = VrrbDbConfig::default();

    if config.db_path() != &vrrbdb_config.path {
        vrrbdb_config.with_path(config.db_path().to_path_buf());
    }

    let db = storage::vrrbdb::VrrbDb::new(vrrbdb_config);

    let vrrbdb_read_handle = db.read_handle();

    let state_module = StateModule::new(state_module::StateModuleConfig { events_tx, db });

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
    mut jsonrpc_events_rx: Receiver<Event>,
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
            .map_err(|err| NodeError::Other(format!("unable to satrt JSON-RPC server: {}", err)))?;

    #[allow(unused_must_use)]
    let jsonrpc_server_handle = Some(tokio::spawn(async move {
        if let Ok(evt) = jsonrpc_events_rx.recv().await {
            if let Event::Stop = evt {
                jsonrpc_server_handle.stop();
                return Ok(());
            }
        }
        Ok(())
    }));

    Ok((jsonrpc_server_handle, resolved_jsonrpc_server_addr))
}

#[allow(unused)]
fn setup_validation_module(
    events_tx: UnboundedSender<DirectedEvent>,
    mut validator_events_rx: Receiver<Event>,
    mempool_read_handle_factory: MempoolReadHandleFactory,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let mut module = ValidatorModule::new();

    let txn_validator_handle =
        tokio::spawn(async move { module.start(&mut validator_events_rx).await });

    Ok(Some(txn_validator_handle))
}

fn setup_mining_module(
    config: &NodeConfig,
    events_tx: UnboundedSender<DirectedEvent>,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
    mut miner_events_rx: Receiver<Event>,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let (_, miner_secret_key) = config.keypair.get_secret_keys();
    let (_, miner_public_key) = config.keypair.get_public_keys();

    let address = Address::new(*miner_public_key).to_string();

    let miner_config = MinerConfig {
        secret_key: *miner_secret_key,
        public_key: *miner_public_key,
        address,
    };

    let miner = miner::Miner::new(miner_config);

    let module_config = MiningModuleConfig {
        miner,
        events_tx,
        vrrbdb_read_handle,
        mempool_read_handle_factory,
    };

    let module = MiningModule::new(module_config);

    let mut miner_module_actor = ActorImpl::new(module);

    let miner_handle = tokio::spawn(async move {
        miner_module_actor
            .start(&mut miner_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(miner_handle))
}

#[allow(unused)]
fn setup_farmer_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

#[allow(unused)]
fn setup_harvester_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

#[allow(unused)]
fn setup_reputation_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

#[allow(unused)]
fn setup_credit_model_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}
