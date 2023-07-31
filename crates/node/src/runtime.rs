use std::sync::{Arc, RwLock};

use block::Block;
use bulldag::graph::BullDag;
use events::{Event, EventPublisher, EventRouter, DEFAULT_BUFFER};
use primitives::Address;
use telemetry::info;
use theater::{Actor, ActorImpl};
use tokio::task::JoinHandle;
use vrrb_config::NodeConfig;
use vrrb_core::claim::Claim;

use crate::{
    api::setup_rpc_api_server,
    consensus::{
        self,
        ConsensusModule,
        ConsensusModuleComponentConfig,
        QuorumMembershipConfig,
        QuorumModuleComponentConfig,
    },
    dag_module::setup_dag_module,
    indexer_module::{self, setup_indexer_module, IndexerModuleConfig},
    mining_module::{MiningModule, MiningModuleComponentConfig},
    network::{NetworkModule, NetworkModuleComponentConfig},
    result::{NodeError, Result},
    state_manager::{StateManager, StateManagerComponentConfig},
    ui::setup_node_gui,
    RuntimeComponent,
    RuntimeComponentManager,
};

pub const PULL_TXN_BATCH_SIZE: usize = 100;

pub async fn setup_runtime_components(
    original_config: &NodeConfig,
    router: &EventRouter,
    events_tx: EventPublisher,
) -> Result<(RuntimeComponentManager, NodeConfig)> {
    let mut config = original_config.clone();

    let vrrbdb_events_rx = router.subscribe(None)?;
    let network_events_rx = router.subscribe(None)?;
    let miner_events_rx = router.subscribe(None)?;
    let jsonrpc_events_rx = router.subscribe(Some("json-rpc-api-control".into()))?;
    let quorum_events_rx = router.subscribe(None)?;
    let consensus_events_rx = router.subscribe(None)?;
    let indexer_events_rx = router.subscribe(None)?;
    let dag_events_rx = router.subscribe(None)?;

    let mut runtime_manager = RuntimeComponentManager::new();

    let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

    let mut membership_config = QuorumMembershipConfig::default();

    let state_component_handle = StateManager::setup(StateManagerComponentConfig {
        events_tx: events_tx.clone(),
        state_events_rx: vrrbdb_events_rx,
        node_config: config.clone(),
        dag: dag.clone(),
    })
    .await?;

    let (state_read_handle, mempool_read_handle_factory) = state_component_handle.data().clone();

    let state_component_handle_label = state_component_handle.label();

    runtime_manager.register_component(
        state_component_handle_label,
        state_component_handle.handle(),
    );

    let network_component_handle = NetworkModule::setup(NetworkModuleComponentConfig {
        config: config.clone(),
        node_id: config.id.clone(),
        events_tx: events_tx.clone(),
        network_events_rx,
        vrrbdb_read_handle: state_read_handle.clone(),
        bootstrap_quorum_config: config.bootstrap_quorum_config.clone(),
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

    let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

    let miner_component = MiningModule::setup(MiningModuleComponentConfig {
        config: config.clone(),
        events_tx: events_tx.clone(),
        vrrbdb_read_handle: state_read_handle.clone(),
        mempool_read_handle_factory: mempool_read_handle_factory.clone(),
        dag: dag.clone(),
        miner_events_rx,
    })
    .await?;

    runtime_manager.register_component(miner_component.label(), miner_component.handle());

    // let dkg_handle = setup_dkg_module(&config, events_tx.clone(),
    // dkg_events_rx)?;

    let public_key = config.keypair.get_miner_public_key().to_owned();

    let signature = Claim::signature_for_valid_claim(
        public_key,
        config.public_ip_address,
        config
            .keypair
            .get_miner_secret_key()
            .secret_bytes()
            .to_vec(),
    )?;

    let claim = Claim::new(
        public_key,
        Address::new(public_key),
        config.public_ip_address,
        signature,
    )
    .map_err(NodeError::from)?;

    //
    // TODO: revisit this
    //
    // events_tx
    //     .send(Event::ClaimCreated(claim.clone()).into())
    //     .await?;

    let quorum_component = consensus::QuorumModule::setup(QuorumModuleComponentConfig {
        events_tx: events_tx.clone(),
        quorum_events_rx,
        vrrbdb_read_handle: state_read_handle.clone(),
        membership_config,
        node_config: config.clone(),
    })
    .await?;

    runtime_manager.register_component(quorum_component.label(), quorum_component.handle());

    let consensus_component = ConsensusModule::setup(ConsensusModuleComponentConfig {
        events_tx: events_tx.clone(),
        node_config: config.clone(),
        vrrbdb_read_handle: state_read_handle.clone(),
        consensus_events_rx,
    })
    .await?;

    runtime_manager.register_component(consensus_component.label(), consensus_component.handle());

    // let (events_tx, events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

    if config.enable_block_indexing {
        let handle = setup_indexer_module(&config, indexer_events_rx, mempool_read_handle_factory)?;
        // TODO: udpate this to return the proper component handle type
        // indexer_handle = Some(handle);
        // TODO: register indexer module handle
    }

    let dag_handle = setup_dag_module(dag, events_tx, dag_events_rx, claim)?;

    let mut node_gui_handle = None;
    if config.gui {
        node_gui_handle = setup_node_gui(&config).await?;
        info!("Node UI started");
    }

    Ok((runtime_manager, config))
}

fn _setup_reputation_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

fn _setup_credit_model_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}
