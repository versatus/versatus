use std::{
    marker::{PhantomData, PhantomPinned},
    net::SocketAddr,
};

use events::{Event, EventPublisher, EventRouter, Topic};
use primitives::{
    KademliaPeerId, NodeType, JSON_RPC_API_TOPIC_STR, NETWORK_TOPIC_STR, RUNTIME_TOPIC_STR,
};
use telemetry::info;
use tokio::{
    sync::mpsc::{channel, UnboundedReceiver},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use vrrb_config::NodeConfig;
use vrrb_core::keypair::{KeyPair, Keypair};
use vrrb_core::node_health_report::NodeHealthReport;

use crate::{
    data_store::DataStore, result::Result, runtime::setup_runtime_components,
    state_reader::StateReader, NodeError, RuntimeComponentManager,
};

/// Node represents a member of the VRRB network and it is responsible for
/// carrying out the different operations permitted within the chain.
#[derive(Debug)]
pub struct Node {
    config: NodeConfig,

    // TODO: make this private
    pub keypair: Keypair,

    cancel_token: CancellationToken,
    runtime_control_handle: JoinHandle<Result<()>>,
}

pub type UnboundedControlEventReceiver = UnboundedReceiver<Event>;

impl Node {
    #[telemetry::instrument(skip(config))]
    pub async fn start(config: NodeConfig) -> Result<Self> {
        // Copy the original config to avoid overwriting the original
        let config = config.clone();

        info!("Launching Node {}", &config.id);

        let keypair = config.keypair.clone();

        let (events_tx, mut events_rx) = channel(events::DEFAULT_BUFFER);

        let mut router = EventRouter::new();
        router.add_topic(Topic::from(JSON_RPC_API_TOPIC_STR), Some(1));
        router.add_topic(Topic::from(NETWORK_TOPIC_STR), Some(1000));
        router.add_topic(Topic::from(RUNTIME_TOPIC_STR), Some(1000));

        let cancel_token = CancellationToken::new();
        let cloned_token = cancel_token.clone();

        let (runtime_component_manager, updated_node_config) =
            setup_runtime_components(&config, &router, events_tx.clone()).await?;

        // TODO: report error from handle
        let router_handle = tokio::spawn(async move { router.start(&mut events_rx).await });
        let runtime_control_handle = tokio::spawn(Self::run_node_main_process(
            config.id.clone(),
            cloned_token,
            events_tx,
            runtime_component_manager,
            router_handle,
        ));

        Ok(Self {
            config: updated_node_config,
            keypair,
            cancel_token,
            runtime_control_handle,
        })
    }

    async fn run_node_main_process(
        id: String,
        cancel_token: CancellationToken,
        events_tx: EventPublisher,
        runtime_component_manager: RuntimeComponentManager,
        router_handle: JoinHandle<()>,
    ) -> Result<()> {
        info!("Node {} is up and running", id);

        // NOTE: wait for stop signal
        cancel_token.cancelled().await;

        events_tx.send(Event::Stop.into()).await?;

        runtime_component_manager.stop().await?;

        router_handle.await?;

        info!("Shutdown complete");

        Ok(())
    }

    /// Stops a [Node].
    /// Returns `true` if it's successfully terminated.
    pub async fn stop(self) -> Result<bool> {
        self.cancel_token.cancel();
        let cancelled = self.cancel_token.is_cancelled();
        self.runtime_control_handle
            .await?
            .map_err(|err| NodeError::Other(err.to_string()))?;

        Ok(cancelled)
    }

    pub async fn config(&self) -> NodeConfig {
        self.config.clone()
    }

    /// Returns a string representation of the Node id
    pub fn id(&self) -> String {
        self.config.id.clone()
    }

    /// Returns the idx of the Node
    pub fn kademlia_peer_id(&self) -> KademliaPeerId {
        self.config.kademlia_peer_id.unwrap_or_default()
    }

    /// Returns the node's type
    pub fn node_type(&self) -> NodeType {
        self.config.node_type
    }

    pub fn is_bootstrap(&self) -> bool {
        matches!(self.node_type(), NodeType::Bootstrap)
    }

    pub fn keypair(&self) -> KeyPair {
        self.keypair.clone()
    }

    pub fn public_address(&self) -> SocketAddr {
        self.config.public_ip_address
    }

    pub fn udp_gossip_address(&self) -> SocketAddr {
        self.config.udp_gossip_address
    }

    pub fn raptorq_gossip_address(&self) -> SocketAddr {
        self.config.raptorq_gossip_address
    }

    pub fn kademlia_liveness_address(&self) -> SocketAddr {
        self.config.kademlia_liveness_address
    }

    pub fn jsonrpc_server_address(&self) -> SocketAddr {
        self.config.jsonrpc_server_address
    }

    /// Reports metrics about the node's health
    pub fn health_check(&self) -> Result<NodeHealthReport> {
        Ok(NodeHealthReport::default())
    }
}
