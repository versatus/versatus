use std::{
    net::SocketAddr,
    process::Command,
    sync::{Arc, RwLock},
    thread,
};

use block::Block;
use bulldag::graph::BullDag;
use crossbeam_channel::Sender;
use events::{Event, EventMessage, EventPublisher, EventRouter, EventSubscriber, DEFAULT_BUFFER};
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
    components::{
        dag_module::DagModule,
        dkg_module::{self, DkgModuleConfig},
        election_module::{
            ElectionModule,
            ElectionModuleConfig,
            MinerElection,
            MinerElectionResult,
            QuorumElection,
            QuorumElectionResult,
        },
        farmer_module::{self, PULL_TXN_BATCH_SIZE},
        harvester_module,
        indexer_module::{self, IndexerModuleConfig},
        mempool_module::{MempoolModule, MempoolModuleComponentConfig},
        mining_module::{MiningModule, MiningModuleConfig},
        network::{NetworkModule, NetworkModuleComponentConfig},
        scheduler::{Job, JobSchedulerController},
        state_module::{StateModule, StateModuleComponentConfig},
    },
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

    let mempool_component_handle = MempoolModule::setup(MempoolModuleComponentConfig {
        events_tx: events_tx.clone(),
        mempool_events_rx,
    })
    .await?;

    let mempool_read_handle_factory = mempool_component_handle.data().clone();

    let state_component_handle = StateModule::setup(StateModuleComponentConfig {
        events_tx: events_tx.clone(),
        state_events_rx: vrrbdb_events_rx,
        node_config: config.clone(),
        dag: dag.clone(),
    })
    .await?;

    let state_read_handle = state_component_handle.data().clone();

    let network_component_handle = NetworkModule::setup(NetworkModuleComponentConfig {
        node_id: config.id.clone(),
        events_tx: events_tx.clone(),
        config: config.clone(),
        network_events_rx,
        node_type: config.node_type,
        vrrbdb_read_handle: state_read_handle.clone(),
    })
    .await?;

    let resolved_network_data = network_component_handle.data();

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

    let mut node_gui_handle = None;
    if config.gui {
        node_gui_handle = setup_node_gui(&config).await?;
    }

    info!("node gui has started");

    let runtime_components = RuntimeComponents {
        node_config: config,
        mempool_handle: Some(mempool_component_handle.handle()),
        state_handle: Some(state_component_handle.handle()),
        gossip_handle: None,
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
                    .args(&["install", "-g", "yarn"])
                    .current_dir("infra/gui")
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
            .args(&["install"])
            .current_dir("infra/gui")
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
            if let Err(err) = Command::new("yarn")
                .args(["dev"])
                .current_dir("infra/gui")
                .spawn()
            {
                telemetry::error!("Failed to spawn UI: {}", err);
            }

            Ok(())
        });

        info!("Finished spawning UI");
        Ok(Some(node_gui_handle))
    } else {
        info!("GUI not enabled");
        Ok(None)
    }
}
