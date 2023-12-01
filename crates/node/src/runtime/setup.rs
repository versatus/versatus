use events::{EventPublisher, EventRouter};
use mempool::MempoolReadHandleFactory;
use primitives::{JSON_RPC_API_TOPIC_STR, NETWORK_TOPIC_STR, RUNTIME_TOPIC_STR};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use vrrb_config::NodeConfig;

use crate::{
    api::setup_rpc_api_server,
    component::NodeRuntimeComponentConfig,
    indexer_module::setup_indexer_module,
    network::{NetworkModule, NetworkModuleComponentConfig},
    node_runtime::NodeRuntime,
    result::Result,
    ui::setup_node_gui,
    RuntimeComponent, RuntimeComponentManager,
};

pub async fn setup_runtime_components(
    original_config: &NodeConfig,
    router: &EventRouter,
    events_tx: EventPublisher,
) -> Result<(
    RuntimeComponentManager,
    NodeConfig,
    VrrbDbReadHandle,
    MempoolReadHandleFactory,
)> {
    let mut config = original_config.clone();

    let runtime_events_rx = router.subscribe(Some(RUNTIME_TOPIC_STR.into()))?;
    let network_events_rx = router.subscribe(Some(NETWORK_TOPIC_STR.into()))?;
    let jsonrpc_events_rx = router.subscribe(Some(JSON_RPC_API_TOPIC_STR.into()))?;
    let indexer_events_rx = router.subscribe(None)?;

    let mut runtime_manager = RuntimeComponentManager::new();

    let node_runtime_component_handle = NodeRuntime::setup(NodeRuntimeComponentConfig {
        config: config.clone(),
        events_tx: events_tx.clone(),
        events_rx: runtime_events_rx,
    })
    .await?;

    let handle_data = node_runtime_component_handle.data();

    let node_config = handle_data.node_config.clone();

    config = node_config;

    let mempool_read_handle_factory = handle_data.mempool_read_handle_factory;
    let state_read_handle = handle_data.state_read_handle;

    runtime_manager.register_component(
        node_runtime_component_handle.label(),
        node_runtime_component_handle.handle(),
    );

    let network_component_handle = NetworkModule::setup(NetworkModuleComponentConfig {
        config: config.clone(),
        node_id: config.id.clone(),
        events_tx: events_tx.clone(),
        network_events_rx,
        vrrbdb_read_handle: state_read_handle.clone(),
        membership_config: config.quorum_config.clone(),
        validator_public_key: config.keypair.validator_public_key_owned(),
    })
    .await?;

    let resolved_network_data = network_component_handle.data();
    let network_component_handle_label = network_component_handle.label();

    runtime_manager.register_component(
        network_component_handle_label,
        network_component_handle.handle(),
    );

    config.kademlia_peer_id = Some(resolved_network_data.kademlia_peer_id);
    config.udp_gossip_address = resolved_network_data.resolved_udp_gossip_address;
    config.raptorq_gossip_address = resolved_network_data.resolved_raptorq_gossip_address;
    config.kademlia_liveness_address = resolved_network_data.resolved_kademlia_liveness_address;

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

    runtime_manager.register_component("API".to_string(), jsonrpc_server_handle);

    if config.enable_block_indexing {
        let _handle = setup_indexer_module(
            &config,
            indexer_events_rx,
            mempool_read_handle_factory.clone(),
        )?;
        // TODO: udpate this to return the proper component handle type
        // indexer_handle = Some(handle);
        // TODO: register indexer module handle
    }

    // TODO: value assigned to `node_gui_handle` is never read.
    let mut _node_gui_handle = None;
    if config.gui {
        _node_gui_handle = setup_node_gui(&config).await?;
        info!("Node UI started");
    }

    Ok((
        runtime_manager,
        config,
        state_read_handle.clone(),
        mempool_read_handle_factory.clone(),
    ))
}
