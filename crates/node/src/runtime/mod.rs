use std::{
    net::SocketAddr,
    process::Command,
    sync::{Arc, RwLock},
    thread,
};

use block::Block;
use bulldag::graph::BullDag;
use crossbeam_channel::{unbounded, Sender};
use events::{Event, EventMessage, EventPublisher, EventRouter, EventSubscriber, DEFAULT_BUFFER};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use miner::{result::MinerError, MinerConfig};
use network::{network::BroadcastEngine, packet::RaptorBroadCastedData};
use primitives::{Address, NodeType, QuorumType::Farmer};
use storage::vrrbdb::{VrrbDbConfig, VrrbDbReadHandle};
use telemetry::info;
use theater::{Actor, ActorImpl};
use tokio::task::JoinHandle;
use validator::validator_core_manager::ValidatorCoreManager;
use vrrb_config::NodeConfig;
use vrrb_core::{
    bloom::Bloom,
    claim::{Claim, ClaimError},
};
use vrrb_grpc::server::{GrpcServer, GrpcServerConfig};
use vrrb_rpc::rpc::{JsonRpcServer, JsonRpcServerConfig};

use self::{
    broadcast_module::{BroadcastModule, BroadcastModuleConfig},
    dag_module::DagModule,
    election_module::{
        ElectionModule, ElectionModuleConfig, MinerElection, MinerElectionResult, QuorumElection,
        QuorumElectionResult,
    },
    indexer_module::IndexerModuleConfig,
    mempool_module::{MempoolModule, MempoolModuleConfig},
    mining_module::{MiningModule, MiningModuleConfig},
    state_module::StateModule,
};
use crate::{
    broadcast_controller::{BroadcastEngineController, BroadcastEngineControllerConfig},
    dkg_module::DkgModuleConfig,
    farmer_module::PULL_TXN_BATCH_SIZE,
    scheduler::{Job, JobSchedulerController},
    NodeError, Result,
};

pub mod broadcast_module;
pub mod credit_model_module;
pub mod dag_module;
pub mod dkg_module;
pub mod election_module;
pub mod farmer_module;
pub mod harvester_module;
pub mod indexer_module;
pub mod mempool_module;
pub mod mining_module;
pub mod reputation_module;
pub mod state_module;
pub mod swarm_module;

pub type RuntimeHandle = Option<JoinHandle<Result<()>>>;
pub type RaptorHandle = Option<thread::JoinHandle<bool>>;
pub type SchedulerHandle = Option<std::thread::JoinHandle<()>>;

impl From<MinerError> for NodeError {
    fn from(_error: MinerError) -> Self {
        NodeError::Other(String::from(
            "Error occurred while creating instance of miner ",
        ))
    }
}
impl From<ClaimError> for NodeError {
    fn from(_error: ClaimError) -> Self {
        NodeError::Other(String::from(
            "Error occurred while creating claim for the node",
        ))
    }
}

#[derive(Debug)]
pub struct RuntimeComponents {
    pub node_config: NodeConfig,
    pub mempool_handle: RuntimeHandle,
    pub state_handle: RuntimeHandle,
    pub gossip_handle: RuntimeHandle,
    pub jsonrpc_server_handle: RuntimeHandle,
    pub miner_handle: RuntimeHandle,
    pub dkg_handle: RuntimeHandle,
    pub miner_election_handle: RuntimeHandle,
    pub quorum_election_handle: RuntimeHandle,
    pub farmer_handle: RuntimeHandle,
    pub harvester_handle: RuntimeHandle,
    pub indexer_handle: RuntimeHandle,
    pub dag_handle: RuntimeHandle,
    pub raptor_handle: RaptorHandle,
    pub scheduler_handle: SchedulerHandle,
    pub grpc_server_handle: RuntimeHandle,
    pub node_gui_handle: RuntimeHandle,
}

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
    let _swarm_module_events_rx = router.subscribe(None)?;

    let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));
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
        dag.clone(),
        mempool_read_handle_factory.clone(),
    )
    .await?;

    let mut gossip_handle = None;
    let (raptor_sender, raptor_receiver) = unbounded::<RaptorBroadCastedData>();
    if !config.disable_networking {
        let (new_gossip_handle, _, gossip_addr) = setup_gossip_network(
            &config,
            events_tx.clone(),
            network_events_rx,
            controller_events_rx,
            state_read_handle.clone(),
            raptor_sender,
        )
        .await?;

        gossip_handle = new_gossip_handle;
        config.udp_gossip_address = gossip_addr;
    }

    let raptor_handle = thread::spawn({
        let events_tx = events_tx.clone();
        move || {
            let events_tx = events_tx.clone();
            loop {
                let events_tx = events_tx.clone();
                if let Ok(data) = raptor_receiver.recv() {
                    match data {
                        RaptorBroadCastedData::Block(block) => {
                            tokio::spawn(async move {
                                let _ = events_tx.send(Event::BlockReceived(block).into()).await;
                            });
                        },
                    }
                }
            }
        }
    });

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

    let (grpc_server_handle, resolved_grpc_server_addr) = setup_grpc_api_server(
        &config,
        events_tx.clone(),
        state_read_handle.clone(),
        mempool_read_handle_factory.clone(),
    )
    .await?;

    info!("gRPC server address started: {}", resolved_grpc_server_addr);

    let dag: Arc<RwLock<BullDag<Block, String>>> = Arc::new(RwLock::new(BullDag::new()));

    let miner_handle = setup_mining_module(
        &config,
        events_tx.clone(),
        state_read_handle.clone(),
        mempool_read_handle_factory.clone(),
        dag.clone(),
        miner_events_rx,
    )?;

    let dkg_handle = setup_dkg_module(&config, events_tx.clone(), dkg_events_rx)?;
    let public_key = *config.keypair.get_miner_public_key();
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

    let miner_election_handle = setup_miner_election_module(
        events_tx.clone(),
        miner_election_events_rx,
        state_read_handle.clone(),
        claim.clone(),
    )?;

    let quorum_election_handle = setup_quorum_election_module(
        &config,
        events_tx.clone(),
        quorum_election_events_rx,
        state_read_handle.clone(),
        claim.clone(),
    )?;

    let (sync_jobs_sender, sync_jobs_receiver) = crossbeam_channel::unbounded::<Job>();
    let (async_jobs_sender, async_jobs_receiver) = crossbeam_channel::unbounded::<Job>();

    let mut farmer_handle = None;
    let mut harvester_handle = None;

    let (events_tx, events_rx) = tokio::sync::mpsc::channel(DEFAULT_BUFFER);

    if config.node_type == NodeType::Farmer {
        farmer_handle = setup_farmer_module(
            &config,
            sync_jobs_sender,
            async_jobs_sender,
            events_tx.clone(),
            farmer_events_rx,
        )?;
    } else {
        // Setup harvester
        harvester_handle = setup_harvester_module(
            &config,
            dag.clone(),
            sync_jobs_sender,
            async_jobs_sender,
            events_tx.clone(),
            events_rx,
            state_read_handle.clone(),
            harvester_events_rx,
        )?
    };

    let valcore_manager =
        ValidatorCoreManager::new(8).map_err(|err| NodeError::Other(err.to_string()))?;

    let mut scheduler = setup_scheduler_module(
        &config,
        sync_jobs_receiver,
        async_jobs_receiver,
        valcore_manager,
        events_tx.clone(),
        state_read_handle.clone(),
    );
    let scheduler_handle = thread::spawn(move || {
        scheduler.execute_sync_jobs();
    });
    let indexer_handle =
        setup_indexer_module(&config, indexer_events_rx, mempool_read_handle_factory)?;

    let dag_handle = setup_dag_module(dag, events_tx, dag_events_rx, claim)?;

    let node_gui_handle = setup_node_gui(&config).await?;

    info!("node gui has started");

    let runtime_components = RuntimeComponents {
        node_config: config,
        mempool_handle,
        state_handle,
        gossip_handle,
        jsonrpc_server_handle,
        miner_handle,
        dkg_handle,
        miner_election_handle,
        quorum_election_handle,
        farmer_handle,
        harvester_handle,
        indexer_handle,
        dag_handle,
        raptor_handle: Some(raptor_handle),
        scheduler_handle: Some(scheduler_handle),
        grpc_server_handle,
        node_gui_handle,
    };

    Ok(runtime_components)
}

async fn setup_gossip_network(
    config: &NodeConfig,
    events_tx: EventPublisher,
    mut network_events_rx: EventSubscriber,
    controller_events_rx: EventSubscriber,
    vrrbdb_read_handle: VrrbDbReadHandle,
    raptor_sender: Sender<RaptorBroadCastedData>,
) -> Result<(
    Option<JoinHandle<Result<()>>>,
    Option<JoinHandle<(Result<()>, Result<()>)>>,
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

    let broadcast_engine = BroadcastEngine::new(config.udp_gossip_address.port(), 32)
        .await
        .map_err(|err| NodeError::Other(format!("unable to setup broadcast engine: {:?}", err)))?;

    let broadcast_resolved_addr = broadcast_engine.local_addr();

    let mut bcast_controller = BroadcastEngineController::new(
        BroadcastEngineControllerConfig::new(broadcast_engine, events_tx.clone()),
    );

    let broadcast_controller_handle = tokio::spawn(async move {
        let broadcast_handle = bcast_controller.listen(controller_events_rx).await;
        let raptor_handle = bcast_controller
            .engine
            .process_received_packets(bcast_controller.engine.raptor_udp_port, raptor_sender)
            .await;

        let raptor_handle = raptor_handle.map_err(NodeError::Broadcast);
        (broadcast_handle, raptor_handle)
    });

    let mut broadcast_module_actor = ActorImpl::new(broadcast_module);

    let broadcast_handle = tokio::spawn(async move {
        broadcast_module_actor
            .start(&mut network_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    info!("Broadcast engine listening on {}", broadcast_resolved_addr);

    Ok((
        Some(broadcast_handle),
        Some(broadcast_controller_handle),
        addr,
    ))
}

async fn setup_state_store(
    config: &NodeConfig,
    events_tx: EventPublisher,
    mut state_events_rx: EventSubscriber,
    dag: Arc<RwLock<BullDag<Block, String>>>,
    _mempool_read_handle_factory: MempoolReadHandleFactory,
) -> Result<(VrrbDbReadHandle, Option<JoinHandle<Result<()>>>)> {
    let mut vrrbdb_config = VrrbDbConfig::default();

    if config.db_path() != &vrrbdb_config.path {
        vrrbdb_config.with_path(config.db_path().to_path_buf());
    }

    let db = storage::vrrbdb::VrrbDb::new(vrrbdb_config);

    let vrrbdb_read_handle = db.read_handle();

    let state_module = StateModule::new(state_module::StateModuleConfig { events_tx, db, dag });

    let mut state_module_actor = ActorImpl::new(state_module);

    let state_handle = tokio::spawn(async move {
        state_module_actor
            .start(&mut state_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    info!("State store is operational");

    Ok((vrrbdb_read_handle, Some(state_handle)))
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

async fn setup_grpc_api_server(
    config: &NodeConfig,
    events_tx: EventPublisher,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
    // mut jsonrpc_events_rx: EventSubscriber,
) -> Result<(Option<JoinHandle<Result<()>>>, SocketAddr)> {
    let grpc_server_config = GrpcServerConfig {
        address: config.grpc_server_address,
        node_type: config.node_type,
        events_tx,
        vrrbdb_read_handle,
        mempool_read_handle_factory,
    };

    let address = grpc_server_config.address;

    let handle = tokio::spawn(async move {
        let _resolved_grpc_server_addr = GrpcServer::run(&grpc_server_config)
            .await
            .map_err(|err| NodeError::Other(format!("unable to start gRPC server, {}", err)))
            .expect("gRPC server to start");
        Ok(())
    });

    info!("gRPC server started at {}", &address);

    Ok((Some(handle), address))
}

fn setup_mining_module(
    config: &NodeConfig,
    events_tx: EventPublisher,
    vrrbdb_read_handle: VrrbDbReadHandle,
    mempool_read_handle_factory: MempoolReadHandleFactory,
    dag: Arc<RwLock<BullDag<Block, String>>>,
    mut miner_events_rx: EventSubscriber,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let (_, miner_secret_key) = config.keypair.get_secret_keys();
    let (_, miner_public_key) = config.keypair.get_public_keys();

    let _address = Address::new(*miner_public_key).to_string();
    let miner_config = MinerConfig {
        secret_key: *miner_secret_key,
        public_key: *miner_public_key,
        ip_address: config.public_ip_address,
        dag,
    };

    let miner = miner::Miner::new(miner_config).map_err(NodeError::from)?;
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

fn setup_dkg_module(
    config: &NodeConfig,
    events_tx: EventPublisher,
    mut dkg_events_rx: EventSubscriber,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let module = dkg_module::DkgModule::new(
        0,
        config.node_type,
        config.keypair.validator_kp.0.clone(),
        DkgModuleConfig {
            quorum_type: Some(Farmer),
            quorum_size: 30,
            /* Need to be decided either will be preconfigured or decided by
             * Bootstrap Node */
            quorum_threshold: 15,
            /* Need to be decided either will be preconfigured or decided
             * by Bootstrap Node */
        },
        config.rendezvous_local_address,
        config.rendezvous_local_address,
        config.udp_gossip_address.port(),
        events_tx,
    );

    if let Ok(dkg_module) = module {
        let mut dkg_module_actor = ActorImpl::new(dkg_module);
        let dkg_handle = tokio::spawn(async move {
            dkg_module_actor
                .start(&mut dkg_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });
        Ok(Some(dkg_handle))
    } else {
        Err(NodeError::Other(String::from(
            "Failed to instantiate dkg module",
        )))
    }
}

fn setup_miner_election_module(
    events_tx: EventPublisher,
    mut miner_election_events_rx: EventSubscriber,
    db_read_handle: VrrbDbReadHandle,
    local_claim: Claim,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let module_config = ElectionModuleConfig {
        db_read_handle,
        events_tx,
        local_claim,
    };
    let module: ElectionModule<MinerElection, MinerElectionResult> =
        { ElectionModule::<MinerElection, MinerElectionResult>::new(module_config) };

    let mut miner_election_module_actor = ActorImpl::new(module);
    let miner_election_module_handle = tokio::spawn(async move {
        miner_election_module_actor
            .start(&mut miner_election_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(miner_election_module_handle))
}

fn setup_quorum_election_module(
    _config: &NodeConfig,
    events_tx: EventPublisher,
    mut quorum_election_events_rx: EventSubscriber,
    db_read_handle: VrrbDbReadHandle,
    local_claim: Claim,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let module_config = ElectionModuleConfig {
        db_read_handle,
        events_tx,
        local_claim,
    };

    let module: ElectionModule<QuorumElection, QuorumElectionResult> =
        { ElectionModule::<QuorumElection, QuorumElectionResult>::new(module_config) };

    let mut quorum_election_module_actor = ActorImpl::new(module);
    let quorum_election_module_handle = tokio::spawn(async move {
        quorum_election_module_actor
            .start(&mut quorum_election_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(quorum_election_module_handle))
}

fn setup_farmer_module(
    config: &NodeConfig,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
    events_tx: EventPublisher,
    mut farmer_events_rx: EventSubscriber,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let module = farmer_module::FarmerModule::new(
        None,
        vec![],
        config.keypair.get_peer_id().into_bytes(),
        // Farmer Node Idx should be updated either by Election or Bootstrap node should assign idx
        0,
        events_tx,
        // Quorum Threshold should be updated on the election,
        1,
        sync_jobs_sender,
        async_jobs_sender,
    );

    let mut farmer_module_actor = ActorImpl::new(module);
    let farmer_handle = tokio::spawn(async move {
        farmer_module_actor
            .start(&mut farmer_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(farmer_handle))
}

fn setup_harvester_module(
    config: &NodeConfig,
    dag: Arc<RwLock<BullDag<Block, String>>>,
    sync_jobs_sender: Sender<Job>,
    async_jobs_sender: Sender<Job>,
    broadcast_events_tx: EventPublisher,
    events_rx: tokio::sync::mpsc::Receiver<EventMessage>,
    vrrb_db_handle: VrrbDbReadHandle,
    mut harvester_events_rx: EventSubscriber,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let module = harvester_module::HarvesterModule::new(
        Bloom::new(PULL_TXN_BATCH_SIZE),
        None,
        vec![],
        events_rx,
        broadcast_events_tx,
        1,
        dag,
        sync_jobs_sender,
        async_jobs_sender,
        vrrb_db_handle,
        config.keypair.clone(),
        config.idx,
    );
    let mut harvester_module_actor = ActorImpl::new(module);
    let harvester_handle = tokio::spawn(async move {
        harvester_module_actor
            .start(&mut harvester_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });
    Ok(Some(harvester_handle))
}

fn setup_dag_module(
    dag: Arc<RwLock<BullDag<Block, String>>>,
    events_tx: EventPublisher,
    mut dag_module_events_rx: EventSubscriber,
    claim: Claim,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let module = DagModule::new(dag, events_tx, claim);

    let mut dag_module_actor = ActorImpl::new(module);
    let dag_module_handle = tokio::spawn(async move {
        dag_module_actor
            .start(&mut dag_module_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(dag_module_handle))
}

fn setup_indexer_module(
    _config: &NodeConfig,
    mut indexer_events_rx: EventSubscriber,
    mempool_read_handle_factory: MempoolReadHandleFactory,
) -> Result<Option<JoinHandle<Result<()>>>> {
    let config = IndexerModuleConfig {
        mempool_read_handle_factory,
    };

    let module = indexer_module::IndexerModule::new(config);

    let mut indexer_module_actor = ActorImpl::new(module);

    let indexer_handle = tokio::spawn(async move {
        indexer_module_actor
            .start(&mut indexer_events_rx)
            .await
            .map_err(|err| NodeError::Other(err.to_string()))
    });

    Ok(Some(indexer_handle))
}

fn setup_scheduler_module(
    config: &NodeConfig,
    sync_jobs_receiver: crossbeam_channel::Receiver<Job>,
    async_jobs_receiver: crossbeam_channel::Receiver<Job>,
    validator_core_manager: ValidatorCoreManager,
    events_tx: EventPublisher,
    vrrbdb_read_handle: VrrbDbReadHandle,
) -> JobSchedulerController {
    JobSchedulerController::new(
        hex::decode(config.keypair.get_peer_id()).unwrap_or(vec![]),
        events_tx,
        sync_jobs_receiver,
        async_jobs_receiver,
        validator_core_manager,
        vrrbdb_read_handle,
    )
}

fn _setup_reputation_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

fn _setup_credit_model_module() -> Result<Option<JoinHandle<Result<()>>>> {
    Ok(None)
}

async fn setup_node_gui(config: &NodeConfig) -> Result<Option<JoinHandle<Result<()>>>> {
    if config.gui {
        info!("Configuring Node {}", &config.id);
        info!("Ensuring environment has required dependencies");

        match Command::new("npm").args(["version"]).status() {
            Ok(_) => info!("NodeJS is installed"),
            Err(e) => {
                return Err(NodeError::Other(format!("NodeJS is not installed: {e}")));
            },
        }

        info!("Ensuring yarn is installed");
        match Command::new("yarn").args(["--version"]).status() {
            Ok(_) => info!("Yarn is installed"),
            Err(e) => {
                let install_yarn = Command::new("npm")
                    .args(["install", "-g", "yarn"])
                    .current_dir("infra/ui")
                    .output();

                match install_yarn {
                    Ok(_) => (),
                    Err(_) => {
                        return Err(NodeError::Other(format!("Failed to install yarn: {e}")));
                    },
                }
            },
        }

        info!("Installing dependencies");
        match Command::new("yarn")
            .args(["install"])
            .current_dir("infra/ui")
            .status()
        {
            Ok(_) => info!("Dependencies installed successfully"),
            Err(e) => {
                return Err(NodeError::Other(format!(
                    "Failed to install dependencies: {e}"
                )));
            },
        }

        info!("Spawning UI");

        let node_gui_handle = tokio::spawn(async move {
            Command::new("yarn")
                .args(["dev"])
                .current_dir("infra/ui")
                .spawn()?;

            Ok(())
        });

        info!("Finished spawning UI");
        Ok(Some(node_gui_handle))
    } else {
        info!("GUI not enabled");
        Ok(None)
    }
}
