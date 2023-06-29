include!("gen/mod.rs");

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use events::{EventPublisher, DEFAULT_BUFFER};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::NodeType;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tokio::sync::mpsc::channel;
use tonic::transport::Server;
use tonic_reflection;

use crate::{node_read::NodeRead, node_write::NodeWrite};

#[derive(Debug, Clone)]
pub struct GrpcServerConfig {
    pub address: SocketAddr,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    pub node_type: NodeType,
    pub events_tx: EventPublisher,
}

#[derive(Debug, Clone)]
pub struct GrpcServer;

impl GrpcServer {
    pub async fn run(config: &GrpcServerConfig) -> anyhow::Result<SocketAddr> {
        let addr = config.address;

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(node_read_service::v1::FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(node_write_service::v1::FILE_DESCRIPTOR_SET)
            .build()
            .map_err(|e| {
                anyhow::Error::msg(format!(
                    "Could not configure reflection for gRPC server: {}",
                    e
                ))
            })?;

        let node_read = NodeRead {
            node_type: config.node_type,
            vrrbdb_read_handle: config.vrrbdb_read_handle.clone(),
            mempool_read_handle_factory: config.mempool_read_handle_factory.clone(),
            events_tx: config.events_tx.clone(),
        };
        let node_read_service = node_read.init();

        let node_write = NodeWrite {
            node_type: config.node_type,
            vrrbdb_read_handle: config.vrrbdb_read_handle.clone(),
            mempool_read_handle_factory: config.mempool_read_handle_factory.clone(),
            events_tx: config.events_tx.clone(),
        };
        let node_write_service = node_write.init();

        Server::builder()
            .add_service(reflection_service)
            .add_service(node_read_service)
            .add_service(node_write_service)
            .serve(addr)
            .await
            .map_err(|e| anyhow::Error::msg(format!("Could not start gRPC server: {}", e)))?;

        Ok(addr)
    }
}

// NOTE: I feel like this needs discussion, default db_path here would be
// different then the actual?
impl Default for GrpcServerConfig {
    fn default() -> GrpcServerConfig {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 50051);
        let mut vrrbdb_config = VrrbDbConfig::default();

        let db_path = primitives::DEFAULT_VRRB_DB_PATH;
        vrrbdb_config.path = PathBuf::from(db_path);

        let vrrbdb = VrrbDb::new(vrrbdb_config);
        let vrrbdb_read_handle = vrrbdb.read_handle();

        let mempool = LeftRightMempool::default();
        let mempool_read_handle_factory = mempool.factory();

        let node_type = NodeType::RPCNode;
        let (events_tx, _) = channel(DEFAULT_BUFFER);

        GrpcServerConfig {
            address,
            vrrbdb_read_handle,
            mempool_read_handle_factory,
            node_type,
            events_tx,
        }
    }
}