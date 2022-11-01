use std::{convert::Infallible, io, net::SocketAddr, time::Duration};

use poem::{
    handler,
    listener::{
        Acceptor, AcceptorExt, RustlsAcceptor, RustlsCertificate, RustlsConfig, RustlsListener,
        TcpAcceptor, TcpListener,
    },
    web::{Json, LocalAddr},
    Endpoint, Route, Server,
};
use poem_openapi::{payload::PlainText, OpenApi, OpenApiService};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};
use vrrb_core::event_router::Event;

#[derive(Debug)]
struct HttpApi;

#[OpenApi]
impl HttpApi {
    /// Healthcheck route
    #[oai(path = "/health", method = "get")]
    async fn index(&self) -> PlainText<&'static str> {
        PlainText("ok")
    }
}

#[derive(Debug, Clone)]
pub struct TlsConfig<T: Into<Vec<u8>>> {
    pub cert: T,
    pub key: T,
}

/// Configuration store for an HttpApiServer
// TODO: implement a builder over this config.
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[derive(Debug, Clone)]
pub struct HttpApiServerConfig<T: Into<Vec<u8>>> {
    pub address: String,
    pub api_title: String,
    pub api_version: String,
    pub server_timeout: Option<Duration>,
    // pub tls_config: Option<TlsConfig>,
    pub tls_config: TlsConfig<T>,
}

/// Configuration store for an HttpApiRouter
// TODO: implement a builder over this config.
// Source<: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[derive(Debug, Clone)]
pub struct HttpApiRouterConfig {
    pub address: SocketAddr,
    pub api_title: String,
    pub api_version: String,
    pub server_timeout: Option<Duration>,
}

/// A JSON-RPC API layer for VRRB nodes.
// pub struct HttpApiServer<A: Acceptor> {
pub struct HttpApiServer {
    // server: Server<Infallible, RustlsAcceptor<TcpAcceptor, ()>>,
    server: Server<Infallible, TcpAcceptor>,
    server_timeout: Option<Duration>,
    app: Route,
}

// impl<A: Acceptor> HttpApiServer<A> {
impl HttpApiServer {
    // TODO: refactor return type into a proper crate specific result
    pub fn new<T: Into<Vec<u8>>>(config: HttpApiServerConfig<T>) -> Result<Self, String> {
        let listener = std::net::TcpListener::bind(config.address.clone())
            .map_err(|err| format!("unable to bind to address: {}", config.address))?;

        let mut acceptor = TcpAcceptor::from_std(listener)
            .map_err(|err| format!("unable to bind to listener on address: {}", config.address))?;

        let tls_config = RustlsConfig::new().fallback(
            RustlsCertificate::new()
                .key(config.tls_config.key)
                .cert(config.tls_config.cert),
        );

        let poem_listener = TcpListener::bind("127.0.0.1:3000");
        poem_listener.rustls();

        // let poem_listener = poem::listener::TcpListener::bind("127.0.0.1:3000").rustls(
        //     RustlsConfig::new().fallback(
        //         RustlsCertificate::new()
        //             .key(tls_config.key)
        //             .cert(tls_config.cert),
        //     ),
        // );

        // // if let Some(tls_config) = config.tls_config {
        // let tls_config = RustlsConfig::new().fallback(
        //     RustlsCertificate::new()
        //         .key(tls_config.key)
        //         .cert(tls_config.cert),
        // );
        //
        // // acceptor = acceptor.rustls(tls_config);
        // // let acceptor =
        // let b = acceptor.rustls(async_stream::stream! {
        //     loop {
        //         if let Ok(tls_config) = load_tls_config() {
        //             yield tls_config;
        //         }
        //         tokio::time::sleep(Duration::from_secs(60)).await;
        //     }
        // });
        // // }

        let address = acceptor.local_addr();
        let address = address
            .get(0)
            .ok_or_else(|| String::from("unable to retrieve the address the server is bound to"))?;

        let address = address
            .as_socket_addr()
            .ok_or_else(|| String::from("unable to retrieve the address the server is bound to"))?;

        let router_config = HttpApiRouterConfig {
            address: address.clone(),
            api_title: config.api_title.clone(),
            api_version: config.api_version.clone(),
            server_timeout: config.server_timeout.clone(),
        };

        let app = Self::create_router(&router_config)?;
        let server = Server::new_with_acceptor(acceptor);
        let server_timeout = config.server_timeout;

        Ok(Self {
            server,
            server_timeout,
            app,
        })
    }

    /// Starts listening for HTTP connections on the configured address.
    /// NOTE: this method needs to consume the instance of HttpApiServer
    pub async fn start(self, ctrl_rx: &mut Receiver<()>) -> io::Result<()> {
        let server_timeout = self.server_timeout;
        let server = self.server;
        let app = self.app;

        server
            .run_with_graceful_shutdown(
                app,
                async {
                    ctrl_rx.recv().await;
                },
                server_timeout,
            )
            .await
    }

    fn create_router(config: &HttpApiRouterConfig) -> Result<Route, String> {
        let openapi_service = OpenApiService::new(
            HttpApi,
            config.api_title.clone(),
            config.api_version.clone(),
        )
        .server(config.address.to_string());

        let ui = openapi_service.swagger_ui();

        Ok(Route::new().nest("/", openapi_service).nest("/docs", ui))
    }
}

#[cfg(test)]
mod tests {
    use poem::{
        get,
        listener::{Acceptor, TcpAcceptor},
        Endpoint,
    };

    use tokio::{signal::unix::SignalKind, sync::mpsc::channel};

    use super::*;
    use poem::test::TestClient;
    use poem::Route;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn index_returns_openapi_docs() {
        let listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let acceptor = TcpAcceptor::from_std(listener).unwrap();
        let addr = acceptor.local_addr();
        let addr = addr.get(0).unwrap();

        let api_title = "Node HTTP API".to_string();
        let api_version = "1.0".to_string();

        let config = HttpApiRouterConfig {
            address: addr.as_socket_addr().unwrap().clone(),
            api_title: "Node HTTP API".into(),
            api_version: "1.0".into(),
            server_timeout: None,
        };

        let app = HttpApiServer::create_router(&config).unwrap();

        let resp = TestClient::new(app).get("/health").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("ok").await;
    }
}
