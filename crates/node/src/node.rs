use std::{net::SocketAddr, path::PathBuf};

use network::network::BroadcastEngine;
use primitives::{NodeIdentifier, NodeIdx, PublicKey, SecretKey};
use state::{NodeState, NodeStateConfig, NodeStateReadHandle};
use telemetry::info;
use theater::{Actor, ActorImpl};
use tokio::{
    sync::{
        broadcast::Receiver,
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};
use trecho::vm::Cpu;
use vrrb_config::NodeConfig;
use vrrb_core::{
    event_router::{DirectedEvent, Event, EventRouter, Topic},
    keypair::KeyPair,
};
use vrrb_rpc::{
    http::HttpApiServerConfig,
    rpc::{JsonRpcServer, JsonRpcServerConfig},
};

use crate::{
    broadcast_module::{BroadcastModule, BroadcastModuleConfig},
    mining_module,
    result::{NodeError, Result},
    validator_module,
    NodeType,
    RuntimeModule,
    RuntimeModuleState,
    StateModule,
    StateModuleConfig,
};

const NUMBER_OF_NETWORK_PACKETS: usize = 32;

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
#[derive(Debug)]
pub struct Node {
    config: NodeConfig,

    // NOTE: core node features
    event_router_handle: JoinHandle<()>,
    running_status: RuntimeModuleState,
    control_rx: UnboundedReceiver<Event>,
    events_tx: UnboundedSender<DirectedEvent>,

    // TODO: make this private
    pub keypair: KeyPair,

    // NOTE: optional node components
    vm: Option<Cpu>,
    state_handle: Option<JoinHandle<Result<()>>>,
    gossip_handle: Option<JoinHandle<Result<()>>>,
    miner_handle: Option<JoinHandle<Result<()>>>,
    txn_validator_handle: Option<JoinHandle<Result<()>>>,
    jsonrpc_server_handle: Option<JoinHandle<Result<()>>>,
}

impl Node {
    /// Initializes and returns a new Node instance
    pub async fn start(config: &NodeConfig, control_rx: UnboundedReceiver<Event>) -> Result<Self> {
        // Copy the original config to avoid overriding the original
        let mut config = config.clone();
        let vm = Some(trecho::vm::Cpu::new());

        let (events_tx, mut events_rx) = unbounded_channel::<DirectedEvent>();

        let mut event_router = Self::setup_event_routing_system();

        let (state_read_handle, state_handle) = Self::setup_state_store(
            &config,
            events_tx.clone(),
            event_router.subscribe(&Topic::State)?,
        )
        .await?;

        let (gossip_handle, gossip_addr) = Self::setup_gossip_network(
            &config,
            events_tx.clone(),
            event_router.subscribe(&Topic::Network)?,
            state_read_handle.clone(),
        )
        .await?;

        config.udp_gossip_address = gossip_addr;

        let (jsonrpc_server_handle, resolved_jsonrpc_server_addr) = Self::setup_rpc_api_server(
            &config,
            events_tx.clone(),
            // event_router.subscribe(&Topic::Network)?,
            state_read_handle.clone(),
        )
        .await?;

        config.jsonrpc_server_address = resolved_jsonrpc_server_addr;

        // TODO: make nodes start with some preconfigured state
        // TODO: make nodes send each other said state with raprtor q

        let txn_validator_handle = Self::setup_validation_module(
            events_tx.clone(),
            event_router.subscribe(&Topic::Transactions)?,
        )?;

        let miner_handle = Self::setup_mining_module(
            //
            events_tx.clone(),
            event_router.subscribe(&Topic::Transactions)?,
        )?;

        // TODO: report error from handle
        let event_router_handle =
            tokio::spawn(async move { event_router.start(&mut events_rx).await });

        Ok(Self {
            config,
            event_router_handle,
            state_handle,
            jsonrpc_server_handle,
            gossip_handle,
            running_status: RuntimeModuleState::Stopped,
            vm,
            control_rx,
            events_tx,
            txn_validator_handle,
            miner_handle,
            keypair: KeyPair::random(),
        })
    }

    pub async fn wait(mut self) -> anyhow::Result<()> {
        // TODO: notify bootstrap nodes that this node is joining the network so they
        // can add it to their peer list

        self.running_status = RuntimeModuleState::Running;

        // NOTE: wait for stop signal
        self.control_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::Other(String::from("failed to receive control signal")))?;

        info!("node received stop signal");

        self.events_tx.send((Topic::Control, Event::Stop))?;

        if let Some(handle) = self.state_handle {
            handle.await??;
            info!("shutdown complete for state management module ");
        }

        if let Some(handle) = self.miner_handle {
            handle.await??;
            info!("shutdown complete for mining module ");
        }

        if let Some(handle) = self.gossip_handle {
            handle.await??;
            info!("shutdown complete for gossip module");
        }

        if let Some(handle) = self.txn_validator_handle {
            handle.await??;
            info!("shutdown complete for mining module ");
        }

        if let Some(handle) = self.jsonrpc_server_handle {
            handle.await??;
            info!("rpc server shut down");
        }

        self.event_router_handle.await?;

        info!("node shutdown complete");

        self.running_status = RuntimeModuleState::Stopped;

        Ok(())
    }

    pub async fn config(&self) -> NodeConfig {
        self.config.clone()
    }

    /// Returns a string representation of the Node id
    pub fn id(&self) -> String {
        self.config.id.clone()
    }

    /// Returns the idx of the Node
    pub fn node_idx(&self) -> u16 {
        self.config.idx
    }

    #[deprecated(note = "use node_idx instead")]
    pub fn get_node_idx(&self) -> u16 {
        self.node_idx()
    }

    /// Returns the node's type
    pub fn node_type(&self) -> NodeType {
        self.config.node_type
    }

    #[deprecated(note = "use node_type instead")]
    pub fn get_node_type(&self) -> NodeType {
        self.node_type()
    }

    pub fn is_bootsrap(&self) -> bool {
        matches!(self.node_type(), NodeType::Bootstrap)
    }

    pub fn status(&self) -> RuntimeModuleState {
        self.running_status.clone()
    }

    pub fn keypair(&self) -> KeyPair {
        self.keypair.clone()
    }

    pub fn udp_gossip_address(&self) -> SocketAddr {
        self.config.udp_gossip_address
    }

    pub fn raprtorq_gossip_address(&self) -> SocketAddr {
        self.config.raptorq_gossip_address
    }

    pub fn bootstrap_node_addresses(&self) -> Vec<SocketAddr> {
        self.config.bootstrap_node_addresses.clone()
    }

    pub fn jsonrpc_server_address(&self) -> SocketAddr {
        self.config.jsonrpc_server_address
    }

    fn setup_event_routing_system() -> EventRouter {
        let mut event_router = EventRouter::new();
        event_router.add_topic(Topic::Control, Some(1));
        event_router.add_topic(Topic::State, Some(1));
        event_router.add_topic(Topic::Transactions, Some(100));
        event_router.add_topic(Topic::Network, Some(100));

        event_router
    }

    async fn setup_gossip_network(
        config: &NodeConfig,
        events_tx: UnboundedSender<DirectedEvent>,
        mut network_events_rx: Receiver<Event>,
        state_handle_factory: NodeStateReadHandle,
        // ) -> Result<(JoinHandle<()>, SocketAddr)> {
    ) -> Result<(Option<JoinHandle<Result<()>>>, SocketAddr)> {
        let bootstrap_node_addresses = config.bootstrap_node_addresses.clone();

        let mut broadcast_module = BroadcastModule::new(BroadcastModuleConfig {
            events_tx: events_tx.clone(),
            state_handle_factory,
            bootstrap_node_addresses,
            udp_gossip_address_port: config.udp_gossip_address.port(),
            raptorq_gossip_address_port: config.raptorq_gossip_address.port(),
            node_type: config.node_type,
            node_id: config.id.as_bytes().to_vec(),
        })
        .await?;

        let addr = broadcast_module.local_addr();

        let broadcast_handle =
            tokio::spawn(async move { broadcast_module.start(&mut network_events_rx).await });

        Ok((Some(broadcast_handle), addr))
    }

    async fn setup_state_store(
        config: &NodeConfig,
        events_tx: UnboundedSender<DirectedEvent>,
        mut state_events_rx: Receiver<Event>,
    ) -> Result<(NodeStateReadHandle, Option<JoinHandle<Result<()>>>)> {
        // TODO: restore state if exists

        let node_state_config = NodeStateConfig {
            path: config.data_dir().to_path_buf(),

            // TODO: read these from config
            serialized_state_filename: None,
            serialized_mempool_filename: None,
            serialized_confirmed_txns_filename: None,
        };

        let node_state = NodeState::new(&node_state_config);

        let mut state_module = StateModule::new(StateModuleConfig {
            events_tx,
            node_state,
        });

        let state_read_handle = state_module.read_handle();

        let mut state_module_actor = ActorImpl::new(state_module);

        let state_handle = tokio::spawn(async move {
            state_module_actor
                .start(&mut state_events_rx)
                .await
                .map_err(|err| NodeError::Other(err.to_string()))
        });

        Ok((state_read_handle, Some(state_handle)))
    }

    async fn setup_rpc_api_server(
        config: &NodeConfig,
        events_tx: UnboundedSender<DirectedEvent>,
        state_read_handle: NodeStateReadHandle,
    ) -> Result<(Option<JoinHandle<Result<()>>>, SocketAddr)> {
        let jsonrpc_server_config = JsonRpcServerConfig {
            address: config.jsonrpc_server_address,
            state_handle_factory: state_read_handle,
            node_type: config.node_type,
            events_tx,
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
}
