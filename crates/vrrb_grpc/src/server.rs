include!("gen/mod.rs");

use std::net::SocketAddr;

use events::{EventPublisher, DEFAULT_BUFFER};
use helloworld::v1::{
    hello_world_service_server::{HelloWorldService, HelloWorldServiceServer},
    SayHelloRequest,
    SayHelloResponse,
};
use mempool::{LeftRightMempool, MempoolReadHandleFactory};
use primitives::NodeType;
use storage::vrrbdb::{VrrbDb, VrrbDbConfig, VrrbDbReadHandle};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Default)]
pub struct MyHelloWorld {}

#[tonic::async_trait]
impl HelloWorldService for MyHelloWorld {
    async fn say_hello(
        &self,
        request: Request<SayHelloRequest>,
    ) -> Result<Response<SayHelloResponse>, Status> {
        let response = SayHelloResponse {
            message: format!("Hello, {}!", request.get_ref().name),
        };
        Ok(Response::new(response))
    }
}

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

        let helloworld = MyHelloWorld::default();
        let svc = HelloWorldServiceServer::new(helloworld);

        if (Server::builder().add_service(svc).serve(addr).await).is_ok() {
            Ok(addr)
        } else {
            Err(anyhow::Error::msg("gRPC server could not start"))
        }
    }
}
