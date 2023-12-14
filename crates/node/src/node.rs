use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use events::{Event, EventPublisher, EventRouter, Topic};
use mempool::MempoolReadHandleFactory;
use metric_exporter::metric_factory::PrometheusFactory;
use primitives::{
    KademliaPeerId, NodeType, JSON_RPC_API_TOPIC_STR, NETWORK_TOPIC_STR, RUNTIME_TOPIC_STR,
};
use prometheus::labels;
use storage::vrrbdb::VrrbDbReadHandle;
use telemetry::info;
use tokio::sync::Mutex;
use tokio::{
    signal,
    sync::mpsc::{channel, UnboundedReceiver},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use vrrb_config::NodeConfig;
use vrrb_core::keypair::{KeyPair, Keypair};
use vrrb_core::node_health_report::NodeHealthReport;

use crate::{
    result::Result, runtime::setup_runtime_components, NodeError, RuntimeComponentManager,
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
    db_read_handle: VrrbDbReadHandle,
    mempool_read_handle: MempoolReadHandleFactory,
}

pub type UnboundedControlEventReceiver = UnboundedReceiver<Event>;

impl Node {
    #[telemetry::instrument(skip(config))]
    pub async fn start(config: NodeConfig) -> Result<Self> {
        // Copy the original config to avoid overwriting the original
        let config = config.clone();

        Self::verify_config(&config)?;

        info!("Launching Node {}", &config.id);

        let keypair = config.keypair.clone();

        let (events_tx, mut events_rx) = channel(events::DEFAULT_BUFFER);

        let mut router = EventRouter::new();
        router.add_topic(Topic::from(JSON_RPC_API_TOPIC_STR), Some(1));
        router.add_topic(Topic::from(NETWORK_TOPIC_STR), Some(1000));
        router.add_topic(Topic::from(RUNTIME_TOPIC_STR), Some(1000));

        let cancel_token = CancellationToken::new();
        let cloned_token = cancel_token.clone();

        //Setting up the prometheus
        let labels = labels! {
                    "service".to_string() => "protocol".to_string(),
                    "source".to_string() => "versatus".to_string(),
        };

        // Prometheus factory for metrics
        let factory = Arc::new(
            PrometheusFactory::new(
                config.prometheus_bind_addr.clone(),
                config.prometheus_bind_port,
                false,
                HashMap::new(),
                config.prometheus_cert_path.clone(),
                config.prometheus_private_key_path.clone(),
                cancel_token.child_token(),
            )
            .unwrap(),
        );

        let (runtime_component_manager, updated_node_config, db_read_handle, mempool_read_handle) =
            setup_runtime_components(
                &config,
                &router,
                events_tx.clone(),
                factory.clone(),
                labels.clone(),
            )
            .await?;

        // TODO: report error from handle
        let router_handle = tokio::spawn(async move { router.start(&mut events_rx).await });
        let runtime_control_handle = tokio::spawn(Self::run_node_main_process(
            config.id.clone(),
            cloned_token,
            events_tx,
            runtime_component_manager,
            router_handle,
            factory.clone(),
        ));

        Ok(Self {
            config: updated_node_config,
            keypair,
            cancel_token,
            runtime_control_handle,
            db_read_handle,
            mempool_read_handle,
        })
    }

    // TODO: implement a more thorough config validation strategy
    fn verify_config(node_config: &NodeConfig) -> Result<()> {
        if node_config.bootstrap_peer_data.is_some() && node_config.bootstrap_config.is_some() {
            return Err(NodeError::ConfigError(
                format!("Node {} config cannot have bootstrap_peer_data and bootstrap_config simultaneously", node_config.id)
                    .to_string(),
            ));
        }
        Ok(())
    }

    async fn run_node_main_process(
        id: String,
        cancel_token: CancellationToken,
        events_tx: EventPublisher,
        runtime_component_manager: RuntimeComponentManager,
        router_handle: JoinHandle<()>,
        factory: Arc<PrometheusFactory>,
    ) -> Result<()> {
        info!("Node {} is up and running", id);

        let mut sighup_receiver = signal::unix::signal(signal::unix::SignalKind::hangup()).unwrap();
        let (sender, receiver) = tokio::sync::mpsc::channel::<()>(100);

        // Assuming async context
        let server = factory.serve(receiver);
        server
            .await
            .map_err(|e| NodeError::Other(format!("Prometheus server failed to start: {:?}", e)))?;

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

    pub fn config(&self) -> NodeConfig {
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
    pub fn prometheus_bind_address(&self) -> String {
        self.config.prometheus_bind_addr.clone()
    }
    pub fn prometheus_bind_port(&self) -> u16 {
        self.config.prometheus_bind_port
    }

    /// Reports metrics about the node's health
    pub fn health_check(&self) -> Result<NodeHealthReport> {
        Ok(NodeHealthReport::default())
    }

    pub fn read_handle(&self) -> VrrbDbReadHandle {
        self.db_read_handle.clone()
    }

    pub fn mempool_read_handle(&self) -> MempoolReadHandleFactory {
        self.mempool_read_handle.clone()
    }
}
