use std::{
    sync::{Arc, RwLock},
    thread,
};

use block::Block;
use bulldag::graph::BullDag;
use events::{Event, EventPublisher, EventRouter, EventSubscriber, DEFAULT_BUFFER};
use mempool::MempoolReadHandleFactory;
use miner::MinerConfig;
use primitives::{Address, NodeType, QuorumType::Farmer};
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use theater::{Actor, ActorImpl};
use tokio::task::JoinHandle;
use validator::validator_core_manager::ValidatorCoreManager;
use vrrb_config::NodeConfig;
use vrrb_core::{bloom::Bloom, claim::Claim};
use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};

use crate::{
    api::setup_rpc_api_server,
    components::{
        consensus::{
            self,
            ConsensusModule,
            ConsensusModuleComponentConfig,
            QuorumModuleComponentConfig,
        },
        dag_module::{setup_dag_module, DagModule},
        // dkg_module::{self, DkgModuleConfig},
        // election_module::{
        //     ElectionModule, ElectionModuleConfig, MinerElection, MinerElectionResult,
        //     QuorumElection, QuorumElectionResult,
        // },
        // farmer_module::{self, PULL_TXN_BATCH_SIZE},
        // harvester_module,
        indexer_module::{self, setup_indexer_module, IndexerModuleConfig},
        mempool_module::{MempoolModule, MempoolModuleComponentConfig},
        mining_module::{MiningModule, MiningModuleComponentConfig},
        network::{NetworkModule, NetworkModuleComponentConfig},
        scheduler::{Job, JobSchedulerController},
        state_module::{StateModule, StateModuleComponentConfig},
        ui::setup_node_gui,
    },
    result::{NodeError, Result},
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
    let quorum_events_rx = router.subscribe(None)?;
    let consensus_events_rx = router.subscribe(None)?;
    let indexer_events_rx = router.subscribe(None)?;
    let dag_events_rx = router.subscribe(None)?;
    let swarm_module_events_rx = router.subscribe(None)?;

    let mut runtime_manager = RuntimeComponentManager::new();

    let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

    let mempool_component_handle = MempoolModule::setup(MempoolModuleComponentConfig {
        events_tx: events_tx.clone(),
        mempool_events_rx,
    })
    .await?;

    let mempool_read_handle_factory = mempool_component_handle.data().clone();
    let mempool_component_handle_label = mempool_component_handle.label();

    runtime_manager.register_component(
        mempool_component_handle_label,
        mempool_component_handle.handle(),
    );

    let state_component_handle = StateModule::setup(StateModuleComponentConfig {
        events_tx: events_tx.clone(),
        state_events_rx: vrrbdb_events_rx,
        node_config: config.clone(),
        dag: dag.clone(),
    })
    .await?;

    let state_read_handle = state_component_handle.data().clone();

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

    events_tx
        .send(Event::ClaimCreated(claim.clone()).into())
        .await?;

    let quorum_component = consensus::QuorumModule::setup(QuorumModuleComponentConfig {
        events_tx: events_tx.clone(),
        quorum_events_rx,
        vrrbdb_read_handle: state_read_handle.clone(),
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

    // let (sync_jobs_sender, sync_jobs_receiver) =
    // crossbeam_channel::unbounded::<Job>(); let (async_jobs_sender,
    // async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

    // let mut farmer_handle = None;
    // let mut harvester_handle = None;

    let (events_tx, events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

    //
    // TODO: re-enable these modules
    //
    //
    // if config.node_type == NodeType::Farmer {
    //     farmer_handle = setup_farmer_module(
    //         &config,
    //         sync_jobs_sender,
    //         async_jobs_sender,
    //         events_tx.clone(),
    //         farmer_events_rx,
    //     )?;
    // } else {
    //     // Setup harvester
    //     harvester_handle = setup_harvester_module(
    //         &config,
    //         dag.clone(),
    //         sync_jobs_sender,
    //         async_jobs_sender,
    //         events_tx.clone(),
    //         events_rx,
    //         state_read_handle.clone(),
    //         harvester_events_rx,
    //     )?
    // };
    //
    // let valcore_manager =
    //     ValidatorCoreManager::new(8).map_err(|err|
    // NodeError::Other(err.to_string()))?;
    //
    // let mut scheduler = setup_scheduler_module(
    //     &config,
    //     sync_jobs_receiver,
    //     async_jobs_receiver,
    //     valcore_manager,
    //     events_tx.clone(),
    //     state_read_handle.clone(),
    // );
    //
    // let scheduler_handle = thread::spawn(move || {
    //     scheduler.execute_sync_jobs();
    // });

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
