use hello_world::{
    greeter_server::{Greeter, GreeterServer},
    HelloReply,
    HelloRequest,
};
use tonic::{
    transport::{
        server::{TcpConnectInfo, TlsConnectInfo},
        Identity,
        Server,
        ServerTlsConfig,
    },
    Request,
    Response,
    Status,
};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = std::path::PathBuf::from_iter([std::env!("CARGO_MANIFEST_DIR"), "data"]);
    let cert = std::fs::read_to_string(data_dir.join("tls/server.pem"))?;
    let key = std::fs::read_to_string(data_dir.join("tls/server.key"))?;

    let identity = Identity::from_pem(cert, key);

    let addr = "127.0.0.1:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .tls_config(ServerTlsConfig::new().identity(identity))?
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
