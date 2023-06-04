use std::net::SocketAddr;

use events::{Event, EventRouter, Topic};
use telemetry::info;
use tokio::{
    sync::mpsc::{channel, UnboundedReceiver},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::error;
use vrrb_config::NodeConfig;
use vrrb_core::keypair::KeyPair;

use crate::{
    result::Result,
    runtime::{setup_runtime_components, RuntimeHandle},
    NodeType,
    RaptorHandle,
    RuntimeModuleState,
    SchedulerHandle,
};

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
pub struct Node {
    config: NodeConfig,

    // NOTE: core node features
    running_status: RuntimeModuleState,

    // TODO: make this private
    pub keypair: KeyPair,

    cancel_token: CancellationToken,
    runtime_control_handle: JoinHandle<Result<()>>,
}

pub type UnboundedControlEventReceiver = UnboundedReceiver<Event>;

impl Node {
    pub async fn start(config: &NodeConfig) -> Result<Self> {
        // Copy the original config to avoid overwriting the original
        let mut config = config.clone();

        info!("Launching Node {}", &config.id);

        let keypair = config.keypair.clone();

        let (events_tx, mut events_rx) = channel(events::DEFAULT_BUFFER);

        let mut router = EventRouter::new();
        router.add_topic(Topic::from("json-rpc-api-control"), Some(1));

        let cancel_token = CancellationToken::new();
        let cloned_token = cancel_token.clone();

        let runtime_components = setup_runtime_components(&config, &router, events_tx).await?;

        let runtime_component_handles = vec![
            // runtime_components.state_handle,
            // runtime_components.mempool_handle,
            // runtime_components.jsonrpc_server_handle,
            // runtime_components.gossip_handle,
            // runtime_components.dkg_handle,
            // runtime_components.miner_handle,
            // runtime_components.miner_election_handle,
            // runtime_components.quorum_election_handle,
            // runtime_components.farmer_handle,
            // runtime_components.harvester_handle,
            // runtime_components.indexer_handle,
            // runtime_components.dag_handle,
            // runtime_components.grpc_server_handle,
            // runtime_components.node_gui_handle,
        ];

        // TODO: report error from handle
        let router_handle = tokio::spawn(async move { router.start(&mut events_rx).await });

        let runtime_control_handle = tokio::spawn(Self::run_node_main_process(
            config.id.clone(),
            cloned_token,
            runtime_component_handles,
            router_handle,
            runtime_components.raptor_handle,
            runtime_components.scheduler_handle,
        ));

        let running_status = RuntimeModuleState::Running;

        info!("Node {} is ready", runtime_components.node_config.id);

        Ok(Self {
            config: runtime_components.node_config,
            running_status,
            keypair,
            cancel_token,
            runtime_control_handle,
        })
    }

    async fn run_node_main_process(
        id: String,
        cancel_token: CancellationToken,
        runtime_component_handles: Vec<RuntimeHandle>,
        router_handle: JoinHandle<()>,
        raptor_handle: RaptorHandle,
        scheduler_handle: SchedulerHandle,
    ) -> Result<()> {
        info!("Node {} is up and running", id);

        // NOTE: wait for stop signal
        cancel_token.cancelled().await;

        for handle in runtime_component_handles {
            if let Some(handle) = handle {
                handle.await??;
                info!("Shutdown complete for handle");
            }
        }

        router_handle.await?;

        if let Some(handle) = raptor_handle {
            if let Err(err) = handle.join() {
                error!("Raptor handle is not shutdown: {err:?}");
            }
        }

        if let Some(handle) = scheduler_handle {
            if let Err(err) = handle.join() {
                error!("Scheduler handle is not shutdown: {err:?}");
            }
        }

        info!("Shutdown complete");

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running_status = RuntimeModuleState::Terminating;
        self.cancel_token.cancel();
        self.running_status = RuntimeModuleState::Stopped;
    }

    pub async fn config(&self) -> NodeConfig {
        self.config.clone()
    }

    pub fn get_public_ip_address(&self) -> SocketAddr {
        self.config.public_ip_address
    }

    /// Returns a string representation of the Node id
    pub fn id(&self) -> String {
        self.config.id.clone()
    }

    /// Returns the idx of the Node
    pub fn node_idx(&self) -> u16 {
        self.config.idx
    }

    /// Returns the node's type
    pub fn node_type(&self) -> NodeType {
        self.config.node_type
    }

    pub fn is_bootstrap(&self) -> bool {
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
}
