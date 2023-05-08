include!("gen/mod.rs");

use helloworld::v1::{
    hello_world_service_server::{HelloWorldService, HelloWorldServiceServer},
    SayHelloRequest,
    SayHelloResponse,
};
use node::v1::{
    node_service_server::{NodeService, NodeServiceServer},
    NodeTypeRequest,
    NodeTypeResponse,
};
use tonic::{transport::Server, Request, Response, Status};

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

#[derive(Debug, Default)]
pub struct Node {}

impl Node {
    pub fn init() -> NodeServiceServer<Node> {
        let node_handler = Node::default();
        let node_service = NodeServiceServer::new(node_handler);
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
            id: "1".to_string(),
            result: "full".to_string(),
        };
        Ok(Response::new(response))
    }
}
