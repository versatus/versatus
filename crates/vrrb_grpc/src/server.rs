include!("gen/mod.rs");

use std::net::SocketAddr;

use events::{EventPublisher, DEFAULT_BUFFER};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::NodeType;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tonic::{transport::Server, Request, Response, Status};
use tonic_reflection;

use crate::handler::MyHelloWorld;

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

        let helloworld_service = MyHelloWorld::init();

        let service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(helloworld::v1::FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        if (Server::builder()
            .add_service(service)
            .add_service(helloworld_service)
            .serve(addr)
            .await)
            .is_ok()
        {
            Ok(addr)
        } else {
            Err(anyhow::Error::msg("gRPC server could not start"))
        }
    }
}
