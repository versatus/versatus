include!("gen/mod.rs");

use helloworld::v1::{
    hello_world_service_server::{HelloWorldService, HelloWorldServiceServer},
    SayHelloRequest,
    SayHelloResponse,
};
use node_type::v1::{
    node_type_service_server::{NodeTypeService, NodeTypeServiceServer},
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
pub struct NodeType {}

impl NodeType {
    pub fn init() -> NodeTypeServiceServer<NodeType> {
        let node_type_handler = NodeType::default();
        let node_type_service = NodeTypeServiceServer::new(node_type_handler);
        return node_type_service;
    }
}

#[tonic::async_trait]
impl NodeTypeService for NodeType {
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
