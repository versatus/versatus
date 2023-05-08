include!("gen/mod.rs");

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use events::{EventPublisher, DEFAULT_BUFFER};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::NodeType;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tokio::sync::mpsc::channel;
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection;

use crate::handler::Node;

#[derive(Debug, Clone)]
pub struct GRPCServerConfig {
    pub address: SocketAddr,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub node_type: NodeType,
    pub events_tx: EventPublisher,
}

#[derive(Debug, Clone)]
pub struct GRPCServer;

impl GRPCServer {
    pub async fn run(config: &GRPCServerConfig) -> anyhow::Result<SocketAddr> {
        let addr = config.address;

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(node::v1::FILE_DESCRIPTOR_SET)
            .build()
            .map_err(|e| anyhow::Error::msg("Could not configure reflection for gRPC server"))?;


        let node = Node {
            node_type: config.node_type,
            vrrbdb_read_handle: config.vrrbdb_read_handle.clone(),
            mempool_read_handle_factory: config.mempool_read_handle_factory.clone(),
            events_tx: config.events_tx.clone(),
        };
        let node_service = node.init();

        Server::builder()
            .add_service(reflection_service)
            .add_service(node_service)
            .serve(addr)
            .await
            .map_err(|e| anyhow::Error::msg("Could not start gRPC server"))?;

        Ok(addr)
    }
}

// I feel like this needs discussion, defulat db_path here would be different
// then the actual?
impl Default for GRPCServerConfig {
    fn default() -> GRPCServerConfig {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50051);
        let mut vrrbdb_config = VrrbDbConfig::default();

        let temp_dir_path = std::env::temp_dir();
        let db_path = temp_dir_path.join(vrrb_core::helpers::generate_random_string());

        vrrbdb_config.path = db_path;

        let vrrbdb = VrrbDb::new(vrrbdb_config);
        let vrrbdb_read_handle = vrrbdb.read_handle();

        let mempool = LeftRightMempool::default();
        let mempool_read_handle_factory = mempool.factory();

        let node_type = NodeType::RPCNode;
        let (events_tx, _) = channel(DEFAULT_BUFFER);

        GRPCServerConfig {
            address,
            vrrbdb_read_handle,
            mempool_read_handle_factory,
            node_type,
            events_tx,
        }
    }
}
