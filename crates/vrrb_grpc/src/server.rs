include!("gen/mod.rs");

use std::net::SocketAddr;

use events::{EventPublisher, DEFAULT_BUFFER};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::NodeType;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection;

use crate::handler::{MyHelloWorld, Node};

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
            .register_encoded_file_descriptor_set(helloworld::v1::FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(node::v1::FILE_DESCRIPTOR_SET)
            .build()
            .map_err(|e| anyhow::Error::msg("Could not configure reflection for gRPC server"))?;

        let helloworld_service = MyHelloWorld::init();

        let node = Node {
            node_type: config.node_type,
            vrrbdb_read_handle: config.vrrbdb_read_handle.to_owned(),
            mempool_read_handle_factory: config.mempool_read_handle_factory.to_owned(),
        };
        let node_service = node.init();

        Server::builder()
            .add_service(reflection_service)
            .add_service(helloworld_service)
            .add_service(node_service)
            .serve(addr)
            .await
            .map_err(|e| anyhow::Error::msg("Could not start gRPC server"))?;

        Ok(addr)
    }
}
