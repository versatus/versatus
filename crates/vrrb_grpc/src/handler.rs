include!("gen/mod.rs");

use helloworld::v1::{
    hello_world_service_server::{HelloWorldService, HelloWorldServiceServer},
    SayHelloRequest,
    SayHelloResponse,
};
use mempool::MempoolReadHandleFactory;
use node::v1::{
    node_service_server::{NodeService, NodeServiceServer},
    NodeTypeRequest,
    NodeTypeResponse,
};
use primitives::NodeType;
use storage::vrrbdb::VrrbDbReadHandle;
use tonic::{transport::Server, Request, Response, Status};

use crate::server::GRPCServerConfig;

#[derive(Debug, Default)]
pub struct MyHelloWorld {}

impl MyHelloWorld {
    pub fn init() -> HelloWorldServiceServer<MyHelloWorld> {
        let helloworld_handler = MyHelloWorld::default();
        let helloworld_service = HelloWorldServiceServer::new(helloworld_handler);
        return helloworld_service;
    }
}

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

#[derive(Debug)]
pub struct Node {
    pub node_type: NodeType,
    pub vrrbdb_read_handle: VrrbDbReadHandle,
    pub mempool_read_handle_factory: MempoolReadHandleFactory,
    // pub events_tx: EventPublisher,
}

impl Node {
    pub fn init(self) -> NodeServiceServer<Node> {
        let node_service = NodeServiceServer::new(self);
        return node_service;
    }
}

#[tonic::async_trait]
impl NodeService for Node {
    async fn get_node_type(
        &self,
        request: Request<NodeTypeRequest>,
    ) -> Result<Response<NodeTypeResponse>, Status> {
        let response = NodeTypeResponse {
            id: (self.node_type as i32).to_string(),
            result: self.node_type.to_string(),
        };
        Ok(Response::new(response))
    }
}
