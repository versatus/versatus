use std::net::SocketAddr;

use events::{Event, EventRouter, Topic};
use telemetry::info;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use trecho::vm::Cpu;
use vrrb_config::NodeConfig;
use vrrb_core::keypair::KeyPair;

use crate::{
    result::{NodeError, Result},
    runtime::{setup_runtime_components, RuntimeHandle},
    NodeType,
    RaptorHandle,
    RuntimeModuleState,
};

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
#[derive(Debug)]
pub struct Node {
    config: NodeConfig,

    // NOTE: core node features
    event_router_handle: JoinHandle<()>,
    running_status: RuntimeModuleState,
    control_rx: UnboundedReceiver<Event>,
    events_tx: UnboundedSender<Event>,

    // TODO: make this private
    pub keypair: KeyPair,

    // NOTE: optional node components
    vm: Option<Cpu>,
    state_handle: RuntimeHandle,
    mempool_handle: RuntimeHandle,
    gossip_handle: RuntimeHandle,
    miner_handle: RuntimeHandle,
    jsonrpc_server_handle: RuntimeHandle,
    dkg_handle: RuntimeHandle,
    miner_election_handle: RuntimeHandle,
    quorum_election_handle: RuntimeHandle,
    farmer_handle: RuntimeHandle,
    harvester_handle: RuntimeHandle,
    indexer_handle: RuntimeHandle,
    dag_handle: RuntimeHandle,
    raptor_handle: RaptorHandle,
}

impl Node {
    /// Initializes and returns a new Node instance
    pub async fn start(config: &NodeConfig, control_rx: UnboundedReceiver<Event>) -> Result<Self> {
        // Copy the original config to avoid overriding the original
        let mut config = config.clone();

        let vm = None;
        let keypair = config.keypair.clone();

        let (events_tx, mut events_rx) = unbounded_channel::<Event>();
        let mut event_router = Self::setup_event_routing_system();

        let mempool_events_rx = event_router.subscribe();
        let vrrbdb_events_rx = event_router.subscribe();
        let network_events_rx = event_router.subscribe();
        let controller_events_rx = event_router.subscribe();
        let miner_events_rx = event_router.subscribe();
        let farmer_events_rx = event_router.subscribe();
        let harvester_events_rx = event_router.subscribe();
        let jsonrpc_events_rx = event_router.subscribe();
        let dkg_events_rx = event_router.subscribe();
        let miner_election_events_rx = event_router.subscribe();
        let quorum_election_events_rx = event_router.subscribe();
        let indexer_events_rx = event_router.subscribe();
        let dag_events_rx = event_router.subscribe();

        let runtime_components = setup_runtime_components(
            &config,
            events_tx.clone(),
            mempool_events_rx,
            vrrbdb_events_rx,
            network_events_rx,
            controller_events_rx,
            miner_events_rx,
            jsonrpc_events_rx,
            dkg_events_rx,
            miner_election_events_rx,
            quorum_election_events_rx,
            farmer_events_rx,
            harvester_events_rx,
            indexer_events_rx,
            dag_events_rx,
        )
        .await?;

        config = runtime_components.node_config;

        // TODO: report error from handle
        let event_router_handle =
            tokio::spawn(async move { event_router.start(&mut events_rx).await });

        Ok(Self {
            config,
            vm,
            event_router_handle,
            state_handle: runtime_components.state_handle,
            mempool_handle: runtime_components.mempool_handle,
            jsonrpc_server_handle: runtime_components.jsonrpc_server_handle,
            gossip_handle: runtime_components.gossip_handle,
            dkg_handle: runtime_components.dkg_handle,
            running_status: RuntimeModuleState::Stopped,
            control_rx,
            events_tx,
            miner_handle: runtime_components.miner_handle,
            keypair,
            miner_election_handle: runtime_components.miner_election_handle,
            quorum_election_handle: runtime_components.quorum_election_handle,
            farmer_handle: runtime_components.farmer_handle,
            harvester_handle: runtime_components.harvester_handle,
            indexer_handle: runtime_components.indexer_handle,
            dag_handle: runtime_components.dag_handle,
            raptor_handle: runtime_components.raptor_handle,
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

        self.events_tx.send(Event::Stop)?;

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
        let mut event_router = EventRouter::new(None);
        event_router.add_topic(Topic::Control, Some(1));
        event_router.add_topic(Topic::State, Some(1));
        event_router.add_topic(Topic::Network, Some(100));
        event_router.add_topic(Topic::Consensus, Some(100));
        event_router.add_topic(Topic::Storage, Some(100));
        event_router.add_topic(Topic::Throttle, Some(100));

        event_router
    }
}
