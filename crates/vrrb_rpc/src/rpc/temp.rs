use std::{fs::File, io::BufReader, net::SocketAddr, sync::Arc};

use axum::{
    extract::Extension,
    handler::post,
    http::{Request, Response},
    Router,
};
use jsonrpsee::{
    http_server::{HttpServerBuilder, TlsConfig},
    raw::{RawRequest, RawRpcServer},
    serde_json::Value,
    server::RpcError,
};
use tokio::signal;
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

// Define the server implementation.
struct MyServer;

impl RawRpcServer for MyServer {
    fn handle_request(&self, req: RawRequest) -> Result<Value, RpcError> {
        match req.method.as_str() {
            "hello" => Ok("world".into()),
            _ => Err(RpcError::MethodNotFound),
        }
    }
}

// Define the RPC endpoint.
async fn rpc(req: Request<Body>, Extension(server): Extension<Arc<MyServer>>) -> Response<Body> {
    // Call the `handle_request` method of the `MyServer` instance.
    let response = server.handle_request(req.into_body()).await;

    // Convert the `Result<Value, RpcError>` to an HTTP response.
    match response {
        Ok(result) => Response::new(Body::from(result.to_string())),
        Err(_) => Response::new(Body::from("RPC error")),
    }
}

// Start the server.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the certificate file.
    let cert_file = &mut BufReader::new(File::open("path/to/certificate.crt")?);
    let cert = rustls::internal::pemfile::certs(&mut cert_file).unwrap();

    // Read the private key file.
    let key_file = &mut BufReader::new(File::open("path/to/private_key.key")?);
    let keys = rustls::internal::pemfile::rsa_private_keys(&mut key_file).unwrap();

    // Create a new `ServerConfig` and set it to use the certificate and private
    // key.
    let mut config = ServerConfig::new(Arc::new(rustls::NoClientAuth::new()));
    config.set_single_cert(cert, keys.remove(0))?;

    // Create a new `TlsAcceptor` with the server configuration.
    let acceptor = TlsAcceptor::from(Arc::new(config));

    // Build the TLS configuration.
    let tls_config = TlsConfig::new()
        .https_only()
        .identity(RustlsConfig::new().acceptor(acceptor));

    // Build the `axum` router.
    let router = Router::new().route("/", post(rpc)).layer(tls_config);

    // Bind the server to the socket address and start the server.
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let server = axum_server::bind_tls(addr, router)?;

    // Wait for the shutdown signal.
    let shutdown_signal = signal::ctrl_c().await?;
    server.stop(shutdown_signal)?;

    Ok(())
}
