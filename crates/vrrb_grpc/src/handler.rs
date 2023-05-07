include!("gen/mod.rs");

use helloworld::v1::{
    hello_world_service_server::{HelloWorldService, HelloWorldServiceServer},
    SayHelloRequest,
    SayHelloResponse,
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
