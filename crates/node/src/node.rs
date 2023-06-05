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
    RuntimeModuleState,
};

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
pub struct Node {
    config: NodeConfig,

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
            runtime_components.state_handle,
            runtime_components.mempool_handle,
            runtime_components.jsonrpc_server_handle,
            runtime_components.gossip_handle,
            runtime_components.dkg_handle,
            runtime_components.miner_handle,
            runtime_components.miner_election_handle,
            runtime_components.quorum_election_handle,
            runtime_components.farmer_handle,
            runtime_components.harvester_handle,
            runtime_components.indexer_handle,
            runtime_components.dag_handle,
            runtime_components.grpc_server_handle,
            runtime_components.node_gui_handle,
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

// /// Initializes and returns a new Node instance
// // pub async fn start(
// //     config: &NodeConfig,
// //     control_rx: UnboundedControlEventReceiver,
// // ) -> Result<Self> {
// //     // Copy the original config to avoid overwriting the original
// //     let mut config = config.clone();
// //
// //     let vm = None;
// //     let keypair = config.keypair.clone();
// //
// //     let (events_tx, mut events_rx) = channel(events::DEFAULT_BUFFER);
// //     let mut router = EventRouter::new();
// //     router.add_topic(Topic::from("json-rpc-api-control"), Some(1));
// //
// //     let runtime_components =
// //         setup_runtime_components(&config, &router,
// events_tx.clone()).await?; //
// //     config = runtime_components.node_config;
// //
// //     // TODO: report error from handle
// //     let router_handle = tokio::spawn(async move { router.start(&mut
// events_rx).await }); //
// //     info!("Node {} is ready", config.id);
// //
// //     Ok(Self {
// //         config,
// //         vm,
// //         keypair,
// //         events_tx,
// //         control_rx,
// //         router_handle,
// //         state_handle: runtime_components.state_handle,
// //         mempool_handle: runtime_components.mempool_handle,
// //         jsonrpc_server_handle: runtime_components.jsonrpc_server_handle,
// //         gossip_handle: runtime_components.gossip_handle,
// //         dkg_handle: runtime_components.dkg_handle,
// //         running_status: RuntimeModuleState::Stopped,
// //         miner_handle: runtime_components.miner_handle,
// //         miner_election_handle: runtime_components.miner_election_handle,
// //         quorum_election_handle: runtime_components.quorum_election_handle,
// //         farmer_handle: runtime_components.farmer_handle,
// //         harvester_handle: runtime_components.harvester_handle,
// //         indexer_handle: runtime_components.indexer_handle,
// //         dag_handle: runtime_components.dag_handle,
// //         raptor_handle: runtime_components.raptor_handle,
// //         scheduler_handle: runtime_components.scheduler_handle,
// //         grpc_server_handle: runtime_components.grpc_server_handle,
// //         node_gui_handle: runtime_components.node_gui_handle,
// //     })
// // }
//
// // pub async fn wait(mut self) -> anyhow::Result<()> {
// //     // TODO: notify bootstrap nodes that this node is joining the network
// so they //     // can add it to their peer list
// //
// //     info!("Launching Node {}", self.id());
// //
// //     self.running_status = RuntimeModuleState::Running;
// //
// //     info!("Node {} is up and running", self.id());
// //
// //     // NOTE: wait for stop signal
// //     self.control_rx
// //         .recv()
// //         .await
// //         .ok_or_else(|| NodeError::Other(String::from("failed to receive
// control signal")))?; //
// //     info!("Node received stop signal");
// //
// //     self.events_tx.send(Event::Stop.into()).await?;
// //
// //     let message = EventMessage::new(Some("json-rpc-api-control".into()),
// Event::Stop); //     self.events_tx.send(message).await?;
// //
// //     if let Some(handle) = self.state_handle {
// //         handle.await??;
// //         info!("Shutdown complete for State module ");
// //     }
// //
// //     if let Some(handle) = self.mempool_handle {
// //         handle.await??;
// //         info!("Shutdown complete for Mempool module ");
// //     }
// //
// //     if let Some(handle) = self.miner_handle {
// //         handle.await??;
// //         info!("Shutdown complete for Mining module ");
// //     }
// //
// //     if let Some(handle) = self.gossip_handle {
// //         handle.await??;
// //         info!("Shutdown complete for Broadcast module");
// //     }
// //
// //     if let Some(handle) = self.dag_handle {
// //         handle.await??;
// //         info!("Shutdown complete for Dag module");
// //     }
// //
// //     if let Some(handle) = self.quorum_election_handle {
// //         handle.await??;
// //         info!("Shutdown complete for Quorum election module");
// //     }
// //
// //     // TODO: refactor this into a tokio task
// //     // if let Some(handle) = self.raptor_handle {
// //     //     handle.join();
// //     //     info!("shutdown complete for raptorq module");
// //     // }
// //
// //     if let Some(handle) = self.jsonrpc_server_handle {
// //         handle.await??;
// //         info!("rpc server shut down");
// //     }
// //
// //     if let Some(handle) = self.node_gui_handle {
// //         handle.await??;
// //         info!("node gui shut down");
// //     }
// //
// //     self.router_handle.await?;
// //
// //     info!("node shutdown complete");
// //
// //     self.running_status = RuntimeModuleState::Stopped;
// //
// //     Ok(())
// // }
